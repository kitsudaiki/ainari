// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use aya::maps::{Array, HashMap as AyaHashMap};
use aya::programs::{SchedClassifier, Xdp, XdpMode};
use aya::{Ebpf, include_bytes_aligned};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::process;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use torii_common::{
    ArpProxy, CONFIG_UPLINK_MODE, IFACE_FLAG_FIP, IfaceConfig, RouteKey, VNI_DEFAULT,
};

use crate::config::CONFIG;
use crate::core::models::{
    ArpProxyPod, FipTargetPod, IfaceConfigPod, Route, RouteFilterPod, RouteKeyPod, RouteTargetPod,
};
use crate::core::routing::build_route_target;
use crate::core::state::GatewayState;
use crate::core::utils::{enable_forwarding, get_ifindex, get_mac_address};

use ainari_api_structs::route_structs::RouteReq;

lazy_static::lazy_static! {
    pub static ref GATEWAY_STATE_HANDLE: Arc<Mutex<GatewayState>> = Arc::new(Mutex::new(init_routing()));
}

/// Loads the eBPF-datapath and builds the initial state of the gateway.
///
/// The compiled eBPF-object is embedded into the binary, so it is loaded from there, its maps are
/// taken over and the programs are attached to the overlay- and underlay-interface. Attaching is
/// skipped for an interface, which does not exist yet, so the torii can also start before the
/// interfaces are configured.
///
/// This is called once to fill the `GATEWAY_STATE_HANDLE`-singleton.
///
/// # Returns
///
/// The state of the gateway with all eBPF-maps, which the endpoints modify at runtime.
///
/// # Panics
///
/// Panics, if the eBPF-object or one of its maps can not be loaded, because the torii can not
/// forward any traffic without its datapath.
pub fn init_routing() -> GatewayState {
    let overlay_iface = &CONFIG.network.overlay_iface;
    let underlay_iface = &CONFIG.network.underlay_iface;

    // IPsec protected traffic is routed by the kernel instead of the eBPF
    // datapath, which requires forwarding to be enabled in this namespace.
    enable_forwarding("/proc/sys/net/ipv4/ip_forward");

    let mut bpf = Ebpf::load(include_bytes_aligned!(concat!(env!("OUT_DIR"), "/torii"))).unwrap();

    // Both forwarding maps are keyed by (vni, destination): one entry per tenant
    // per address, which is what allows the same address in several tenants.
    let route_map_data = bpf.take_map("ROUTE_MAP").expect("Missing ROUTE_MAP");
    let route_map: AyaHashMap<_, RouteKeyPod, RouteTargetPod> =
        AyaHashMap::try_from(route_map_data).unwrap();

    // Floating IPs are unique across tenants, so only the reverse direction needs
    // the tenant in its key.
    let fip_dnat_map_data = bpf.take_map("FIP_DNAT_MAP").expect("Missing FIP_DNAT_MAP");
    let fip_dnat_map: AyaHashMap<_, u32, FipTargetPod> =
        AyaHashMap::try_from(fip_dnat_map_data).unwrap();

    let fip_snat_map_data = bpf.take_map("FIP_SNAT_MAP").expect("Missing FIP_SNAT_MAP");
    let fip_snat_map: AyaHashMap<_, RouteKeyPod, u32> =
        AyaHashMap::try_from(fip_snat_map_data).unwrap();

    let arp_proxy_map_data = bpf
        .take_map("ARP_PROXY_MAP")
        .expect("Missing ARP_PROXY_MAP");
    let arp_proxy_map: AyaHashMap<_, u32, ArpProxyPod> =
        AyaHashMap::try_from(arp_proxy_map_data).unwrap();

    let iface_map_data = bpf.take_map("IFACE_MAP").expect("Missing IFACE_MAP");
    let iface_map: AyaHashMap<_, u32, IfaceConfigPod> =
        AyaHashMap::try_from(iface_map_data).unwrap();

    let filter_map_data = bpf.take_map("FILTER_MAP").expect("Missing FILTER_MAP");
    let filter_map: AyaHashMap<_, RouteKeyPod, RouteFilterPod> =
        AyaHashMap::try_from(filter_map_data).unwrap();

    // The egress filters are keyed by the TAP device the VM sends on, not by an address the VM
    // could choose itself.
    let egress_filter_map_data = bpf
        .take_map("EGRESS_FILTER_MAP")
        .expect("Missing EGRESS_FILTER_MAP");
    let egress_filter_map: AyaHashMap<_, u32, RouteFilterPod> =
        AyaHashMap::try_from(egress_filter_map_data).unwrap();

    // STATIC eBPF ATTACHMENT (Safely skips if interface doesn't exist yet)
    let overlay: &mut Xdp = bpf
        .program_mut("overlay_ingress")
        .unwrap()
        .try_into()
        .unwrap();
    overlay.load().unwrap();
    if get_ifindex(overlay_iface) > 0 {
        overlay.attach(overlay_iface, XdpMode::Skb).unwrap();
        println!("Attached overlay_ingress to {}", overlay_iface);
    } else {
        println!(
            "Waiting for dynamic TAP creation. Skipping initial overlay attach for {}",
            overlay_iface
        );
    }

    let underlay: &mut Xdp = bpf
        .program_mut("underlay_ingress")
        .unwrap()
        .try_into()
        .unwrap();
    underlay.load().unwrap();
    if get_ifindex(underlay_iface) > 0 {
        underlay.attach(underlay_iface, XdpMode::Skb).unwrap();
        println!("Attached underlay_ingress to {}", underlay_iface);
    } else {
        println!("Warning: Underlay interface {} not found.", underlay_iface);
    }

    // The filter program for the egress of the TAP devices is only loaded here. It is attached
    // to every TAP device on its registration.
    let tap_egress: &mut SchedClassifier =
        bpf.program_mut("tap_egress").unwrap().try_into().unwrap();
    tap_egress.load().unwrap();

    let mut state = GatewayState {
        routes: HashMap::new(),
        floating_ips: HashMap::new(),
        taps: HashMap::new(),
        tap_xdp_links: HashMap::new(),
        tap_tc_links: HashMap::new(),
        crypto_keys: HashMap::new(),
        connections: HashMap::new(),
        filters: HashMap::new(),
        route_map,
        filter_map,
        egress_filter_map,
        fip_dnat_map,
        fip_snat_map,
        arp_proxy_map,
        iface_map,
        bpf, // Retain Ebpf context for dynamic API attachments
    };

    // UPLINK: the gateway at the edge of the network serves the floating IPs and sends
    // everything, which leaves the virtual network, to the next hop behind its uplink. A gateway
    // without an uplink keeps the uplink maps empty and only routes within the network.
    if let Some((uplink_iface, next_hop)) = CONFIG.network.uplink() {
        if let Err(e) = setup_uplink(&mut state, uplink_iface, next_hop, overlay_iface) {
            log::error!("Failed to set up the uplink: {e}");
            process::exit(1);
        }
    }

    // DEFAULT ROUTE: a gateway without an own uplink sends everything, which is not addressed to
    // one of its virtual machines, through the underlay to the gateway at the edge of the network
    if let Some(gateway_ip) = CONFIG.network.default_gateway_ip {
        let underlay_iface = &CONFIG.network.underlay_iface;
        if let Err(e) = setup_default_route(&mut state, gateway_ip, underlay_iface) {
            log::error!("Failed to set up the default route: {e}");
            process::exit(1);
        }
    }

    state
}

/// Derives the UUID of a route, which the gateway builds from its own config.
///
/// These routes are created again with every start, so their UUID is derived from their
/// `(vni, dest_ip)`-key instead of being random. That way it stays the same across restarts, and
/// everything persisted for the route - like an update of it - still finds it.
///
/// # Arguments
/// * `vni` - Tenant of the route
/// * `dest_ip` - Destination address of the route
///
/// # Returns
/// A version 8 UUID, which carries the tenant and the destination of the route
pub fn config_route_uuid(vni: u32, dest_ip: Ipv4Addr) -> Uuid {
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&vni.to_be_bytes());
    // bytes 6 and 8 carry the version and the variant, so the address is placed behind them
    bytes[10..14].copy_from_slice(&dest_ip.octets());
    uuid::Builder::from_custom_bytes(bytes).into_uuid()
}

/// Points the default route of this gateway to the gateway at the edge of the network.
///
/// Everything, which doesn't match one of the routes of this gateway, is encapsulated and sent
/// through the underlay to that gateway, which forwards it to the outside.
///
/// # Arguments
/// * `state` - State of the gateway, which the route is added to
/// * `gateway_ip` - Underlay-address of the gateway at the edge of the network
/// * `underlay_iface` - Interface, which carries the traffic to the other gateways
///
/// # Returns
/// `Ok(())` once the default route is programmed, otherwise the reason why it could not be set up
fn setup_default_route(
    state: &mut GatewayState,
    gateway_ip: Ipv4Addr,
    underlay_iface: &str,
) -> Result<(), anyhow::Error> {
    let req = RouteReq {
        dest_ip: Ipv4Addr::UNSPECIFIED,
        target_iface: underlay_iface.to_owned(),
        vni: VNI_DEFAULT,
        gateway_ip: Some(gateway_ip),
        next_hop_ip: None,
        next_hop_mac: None,
        encrypted: false,
    };
    let target = build_route_target(&req, &state.taps).map_err(anyhow::Error::msg)?;
    state.route_map.insert(
        RouteKeyPod(RouteKey::default_route(req.vni)),
        RouteTargetPod(target),
        0,
    )?;

    let route_uuid = config_route_uuid(req.vni, Ipv4Addr::UNSPECIFIED);
    state.routes.insert(
        route_uuid,
        Route {
            uuid: route_uuid,
            vni: req.vni,
            dest_ip: Ipv4Addr::UNSPECIFIED,
            target_iface: req.target_iface,
            gateway_ip: req.gateway_ip,
            next_hop_ip: None,
            next_hop_mac: None,
            encrypted: false,
        },
    );

    log::info!("default route points to the gateway {gateway_ip}");

    Ok(())
}

/// Turns this gateway into the edge of the network by activating its uplink.
///
/// The overlay program is attached to the uplink (unless it already sits there
/// as the overlay interface), the uplink is registered in `UPLINK_MAP` together
/// with its MAC - which the ARP responder hands out for the floating IPs - and
/// the uplink mode of the datapath is switched on. At last the routes towards
/// the next hop and the default route are pointed at the uplink, so everything
/// that is not a VM leaves the virtual network there.
///
/// # Arguments
/// * `state` - The freshly initialized gateway state
/// * `uplink_iface` - Name of the interface facing the outside
/// * `next_hop` - Address behind the uplink all outgoing traffic is sent to
/// * `overlay_iface` - Name of the interface the overlay program was attached to at startup
///
/// # Returns
/// `Ok(())` once the uplink is active, otherwise the reason why it could not be set up
fn setup_uplink(
    state: &mut GatewayState,
    uplink_iface: &str,
    next_hop: Ipv4Addr,
    overlay_iface: &str,
) -> Result<(), anyhow::Error> {
    let ifindex = get_ifindex(uplink_iface);
    if ifindex == 0 {
        anyhow::bail!("Uplink interface {} not found", uplink_iface);
    }

    if uplink_iface != overlay_iface {
        let overlay: &mut Xdp = state
            .bpf
            .program_mut("overlay_ingress")
            .expect("Missing overlay_ingress")
            .try_into()?;
        overlay.attach(uplink_iface, XdpMode::Skb)?;
    }

    let mut uplinks: AyaHashMap<_, u32, ArpProxyPod> =
        AyaHashMap::try_from(state.bpf.map_mut("UPLINK_MAP").expect("Missing UPLINK_MAP"))?;
    let uplink = ArpProxy {
        mac: get_mac_address(uplink_iface),
        _pad: [0; 2],
        vm_ip: 0,
    };
    uplinks.insert(ifindex, ArpProxyPod(uplink), 0)?;

    // The uplink is the one port that faces the outside world, so it is where a
    // floating IP is allowed to name the tenant of a packet. It belongs to the
    // shared tenant itself: everything behind it is outside the virtual network.
    state.iface_map.insert(
        ifindex,
        IfaceConfigPod(IfaceConfig {
            vni: VNI_DEFAULT,
            flags: IFACE_FLAG_FIP,
        }),
        0,
    )?;

    let mut config: Array<_, u32> = Array::try_from(
        state
            .bpf
            .map_mut("GATEWAY_CONFIG")
            .expect("Missing GATEWAY_CONFIG"),
    )?;
    config.set(CONFIG_UPLINK_MODE, 1, 0)?;

    for dest_ip in [next_hop, Ipv4Addr::UNSPECIFIED] {
        let req = RouteReq {
            dest_ip,
            target_iface: uplink_iface.to_owned(),
            vni: VNI_DEFAULT,
            gateway_ip: None,
            next_hop_ip: Some(next_hop),
            next_hop_mac: None,
            encrypted: false,
        };
        let target = build_route_target(&req, &state.taps).map_err(anyhow::Error::msg)?;
        state.route_map.insert(
            RouteKeyPod(RouteKey::new(req.vni, u32::from(dest_ip))),
            RouteTargetPod(target),
            0,
        )?;

        let route_uuid = config_route_uuid(req.vni, dest_ip);
        state.routes.insert(
            route_uuid,
            Route {
                uuid: route_uuid,
                vni: req.vni,
                dest_ip,
                target_iface: req.target_iface,
                gateway_ip: None,
                next_hop_ip: req.next_hop_ip,
                next_hop_mac: None,
                encrypted: false,
            },
        );
    }

    log::warn!(
        "Single node setup (development only): {} is the uplink towards {}, floating IPs are served on it",
        uplink_iface,
        next_hop
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_route_uuids_are_stable_and_unique_per_key() {
        let default_route = config_route_uuid(VNI_DEFAULT, Ipv4Addr::UNSPECIFIED);
        assert_eq!(
            default_route,
            config_route_uuid(VNI_DEFAULT, Ipv4Addr::UNSPECIFIED)
        );
        assert_eq!(default_route.get_version(), Some(uuid::Version::Custom));

        let next_hop = config_route_uuid(VNI_DEFAULT, Ipv4Addr::new(10, 0, 0, 1));
        let other_tenant = config_route_uuid(5, Ipv4Addr::new(10, 0, 0, 1));
        assert_ne!(default_route, next_hop);
        assert_ne!(next_hop, other_tenant);
    }
}
