//! Translation of route requests into the targets consumed by the eBPF maps.
//!
//! This is where the decision is made how a destination is reached: through the
//! L2-in-UDP overlay, by handing the packet to the kernel for IPsec, or by
//! delivering it locally on a TAP or the veth uplink.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use torii_common::{ROUTE_ACTION_ENCAP, ROUTE_ACTION_KERNEL, ROUTE_ACTION_LOCAL, RouteTarget};

use crate::core::crypto::install_block_policies;
use crate::core::models::TapInfo;
use crate::core::utils::{
    get_arp_mac, get_ifindex, get_local_ip, get_mac_address, parse_mac, run_ip,
};

use ainari_api_structs::route_structs::*;

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
        let dest = format!("{}/32", req.dest_ip);
        run_ip(&[
            "route",
            "replace",
            &dest,
            "via",
            &gateway_ip.to_string(),
            "dev",
            &req.target_iface,
        ])?;
        install_block_policies(req.dest_ip)?;
    } else if let Some(gateway_ip) = req.gateway_ip {
        action = ROUTE_ACTION_ENCAP;
        encap_dst_ip = u32::from(gateway_ip);
        encap_dst_mac = get_arp_mac(gateway_ip);

        let local_ip = get_local_ip("eth0").unwrap_or(Ipv4Addr::UNSPECIFIED);
        encap_src_ip = u32::from(local_ip);
        encap_src_mac = get_mac_address("eth0");
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
