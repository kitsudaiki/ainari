//! Translation of route requests into the targets consumed by the eBPF maps.
//!
//! This is where the decision is made how a destination is reached: through the
//! VXLAN overlay, by handing the packet to the kernel for IPsec, or by
//! delivering it locally on a TAP or the veth uplink.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use torii_common::{ROUTE_ACTION_ENCAP, ROUTE_ACTION_KERNEL, ROUTE_ACTION_LOCAL, RouteTarget};

use uuid::Uuid;

use crate::config::CONFIG;
use crate::core::crypto::install_block_policies;
use crate::core::models::TapInfo;
use crate::core::state::GatewayState;
use crate::core::utils::{
    get_arp_mac, get_ifindex, get_local_ip, get_mac_address, get_next_hop, parse_mac, run_ip,
    with_table,
};

use ainari_api_structs::route_structs::*;

/// Checks that a route may be created in the tenant it names.
///
/// Two things are verified: a local route must not point at a TAP of another
/// tenant, and an encrypted route must not collide with an encrypted route of
/// another tenant. The second one is a limitation of the kernel rather than of
/// this datapath - xfrm selectors are plain address pairs with no room for a
/// tenant - so instead of silently overwriting the policies of the first tenant
/// the second one is refused.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `req` - The route request being processed
/// * `skip` - UUID of the route being updated, so it does not collide with itself
///
/// # Returns
/// `Ok(())` when the route is safe to program, otherwise a message for the client
pub fn check_route_tenant(
    st: &GatewayState,
    req: &RouteReq,
    skip: Option<Uuid>,
) -> Result<(), String> {
    if req.gateway_ip.is_none()
        && let Some(tap) = st.taps.get(&req.target_iface)
        && tap.vni != req.vni
    {
        return Err(format!(
            "{} belongs to tenant {}, a route in tenant {} must not deliver into it",
            req.target_iface, tap.vni, req.vni
        ));
    }

    if req.encrypted
        && let Some(other) = st.routes.values().find(|route| {
            Some(route.uuid) != skip
                && route.encrypted
                && route.dest_ip == req.dest_ip
                && route.vni != req.vni
        })
    {
        return Err(format!(
            "{} is already reached over IPsec in tenant {}. The kernel's xfrm selectors carry no \
             tenant, so the same address cannot be protected in tenant {} as well",
            req.dest_ip, other.vni, req.vni
        ));
    }

    Ok(())
}

/// Determines the link layer next hop a locally delivered packet is addressed to.
///
/// The gateway acts as a real L3 hop for the VMs, so packets leaving a local
/// interface must carry the MAC of whoever sits on the other end of that link.
/// The address is taken from the first source that can provide it:
///
/// 1. an explicit `next_hop_mac` in the request
/// 2. the VM registered with the target TAP device (the normal case)
/// 3. an ARP lookup of `next_hop_ip`, or of `dest_ip` if none was given -
///    only usable on numbered interfaces such as the veth uplink
///
/// # Arguments
/// * `req` - The route request that is being programmed
/// * `taps` - Snapshot of the TAP devices managed by this gateway
///
/// # Returns
/// A 6-byte MAC address, or all zeros when no next hop could be determined
/// (in which case the datapath forwards the frame without rewriting L2)
pub fn resolve_next_hop_mac(req: &RouteReq, taps: &HashMap<String, TapInfo>) -> [u8; 6] {
    if let Some(mac) = req.next_hop_mac.as_deref().and_then(parse_mac) {
        return mac;
    }

    // TAP devices are unnumbered, so ARP is not an option here: the MAC of the
    // attached VM is taken from the registration done by /interfaces/tap.
    if let Some(info) = taps.get(&req.target_iface) {
        return info.vm_mac.unwrap_or([0u8; 6]);
    }

    let probe = req.next_hop_ip.unwrap_or(req.dest_ip);

    if !probe.is_unspecified() {
        let mac = get_arp_mac(probe);
        // get_arp_mac falls back to broadcast when resolution fails.
        if mac != [0xff; 6] {
            return mac;
        }
    }

    [0u8; 6]
}

/// Translates a route request into the `RouteTarget` consumed by the eBPF maps.
///
/// Routes carrying a `gateway_ip` are programmed as tunnel routes (`action == 1`)
/// and get the underlay addresses of the remote gateway attached. All other
/// routes are local deliveries (`action == 0`) and get the link layer addresses
/// of the outgoing interface and of its next hop, which is what keeps the TAP
/// devices free of any IP configuration.
///
/// The tenant of the request is carried into the target. On a tunnel route it
/// ends up in the VXLAN header of every encapsulated packet, which is the only
/// thing that tells the receiving gateway which of its tenants the inner address
/// belongs to. On a kernel route it selects the routing table the destination is
/// programmed into.
///
/// # Arguments
/// * `req` - The route request to translate
/// * `taps` - Snapshot of the TAP devices managed by this gateway
///
/// # Returns
/// A `Result` holding the populated `RouteTarget`, or an error message
pub fn build_route_target(
    req: &RouteReq,
    taps: &HashMap<String, TapInfo>,
) -> Result<RouteTarget, String> {
    let ifindex = get_ifindex(&req.target_iface);
    if ifindex == 0 {
        return Err(format!("Interface {} not found", req.target_iface));
    }

    let mut action = ROUTE_ACTION_LOCAL;
    let mut encap_dst_ip = 0;
    let mut encap_dst_mac = [0u8; 6];
    let mut encap_src_ip = 0;
    let mut encap_src_mac = [0u8; 6];
    let mut l2_dst_mac = [0u8; 6];
    let mut l2_src_mac = [0u8; 6];

    if req.encrypted {
        // IPsec protected destination. The packet leaves the eBPF datapath and
        // is routed by the kernel, so the only thing to prepare here is the
        // route towards the gateway that hosts the remote VM plus the policies
        // that keep unprotected traffic from taking the same path.
        action = ROUTE_ACTION_KERNEL;
        let gateway_ip = match req.gateway_ip {
            Some(gateway_ip) => gateway_ip,
            None => {
                return Err(
                    "encrypted routes need the underlay address of the remote gateway in gateway_ip"
                        .to_string(),
                );
            }
        };
        // The kernel has no VNI, so the destination goes into the routing table of
        // its tenant. Traffic reaches that table through the `ip rule` the TAP of
        // the sending VM installed.
        let dest = format!("{}/32", req.dest_ip);
        let gateway_ip_str = gateway_ip.to_string();
        let table = CONFIG.network.tenant_table(req.vni);
        let mut args = vec![
            "route",
            "replace",
            dest.as_str(),
            "via",
            gateway_ip_str.as_str(),
            "dev",
            req.target_iface.as_str(),
        ];
        with_table(&mut args, &table);
        run_ip(&args)?;

        // The ESP packet built by the xfrm stack is addressed to the peer gateway
        // and needs a route of its own. In the main table the underlay subnet
        // route already covers it; a tenant table holds nothing but what is put
        // there, so the peer is stated explicitly.
        let peer = format!("{}/32", gateway_ip_str);
        let mut peer_args = vec![
            "route",
            "replace",
            peer.as_str(),
            "dev",
            req.target_iface.as_str(),
        ];
        with_table(&mut peer_args, &table);
        run_ip(&peer_args)?;

        install_block_policies(req.dest_ip)?;
    } else if let Some(gateway_ip) = req.gateway_ip {
        action = ROUTE_ACTION_ENCAP;
        encap_dst_ip = u32::from(gateway_ip);
        // The remote gateway is not necessarily on the same link, so the frame
        // is addressed to the router in between, if there is one.
        let next_hop = get_next_hop(gateway_ip, &CONFIG.network.underlay_iface);
        encap_dst_mac = get_arp_mac(next_hop);

        let local_ip =
            get_local_ip(&CONFIG.network.underlay_iface).unwrap_or(Ipv4Addr::UNSPECIFIED);
        encap_src_ip = u32::from(local_ip);
        encap_src_mac = get_mac_address(&CONFIG.network.underlay_iface);
    } else {
        // Local delivery: rewrite the frame onto the target link. The source
        // becomes the router port itself, the destination its next hop.
        l2_src_mac = match taps.get(&req.target_iface) {
            Some(info) => info.tap_mac,
            None => get_mac_address(&req.target_iface),
        };
        l2_dst_mac = resolve_next_hop_mac(req, taps);

        // Without a next hop the frame keeps the MAC the sender used, which the
        // receiving side will normally discard. Say so instead of silently
        // programming a route that black-holes traffic.
        if l2_dst_mac == [0u8; 6] {
            println!(
                "Warning: no link layer next hop for {} via {}. Frames will be forwarded \
                 unmodified; pass next_hop_ip or next_hop_mac for this route.",
                req.dest_ip, req.target_iface
            );
        }
    }

    Ok(RouteTarget {
        action,
        ifindex,
        vni: req.vni,
        encap_dst_ip,
        encap_dst_mac,
        _pad1: [0; 2],
        encap_src_ip,
        encap_src_mac,
        _pad2: [0; 2],
        l2_dst_mac,
        _pad3: [0; 2],
        l2_src_mac,
        _pad4: [0; 2],
    })
}
