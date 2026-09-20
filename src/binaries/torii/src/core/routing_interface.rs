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
use aya::programs::{Xdp, XdpFlags};
use aya::{Bpf, include_bytes_aligned};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::process;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use torii_common::{ArpProxy, CONFIG_UPLINK_MODE};

use crate::config::CONFIG;
use crate::core::models::{ArpProxyPod, Route, RouteFilterPod, RouteTargetPod};
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

    let mut bpf = Bpf::load(include_bytes_aligned!(concat!(env!("OUT_DIR"), "/torii"))).unwrap();

    let route_map_data = bpf.take_map("ROUTE_MAP").expect("Missing ROUTE_MAP");
    let route_map: AyaHashMap<_, u32, RouteTargetPod> =
        AyaHashMap::try_from(route_map_data).unwrap();

    let fip_dnat_map_data = bpf.take_map("FIP_DNAT_MAP").expect("Missing FIP_DNAT_MAP");
    let fip_dnat_map: AyaHashMap<_, u32, u32> = AyaHashMap::try_from(fip_dnat_map_data).unwrap();

    let fip_snat_map_data = bpf.take_map("FIP_SNAT_MAP").expect("Missing FIP_SNAT_MAP");
    let fip_snat_map: AyaHashMap<_, u32, u32> = AyaHashMap::try_from(fip_snat_map_data).unwrap();

    let arp_proxy_map_data = bpf
        .take_map("ARP_PROXY_MAP")
        .expect("Missing ARP_PROXY_MAP");
    let arp_proxy_map: AyaHashMap<_, u32, ArpProxyPod> =
        AyaHashMap::try_from(arp_proxy_map_data).unwrap();

    let filter_map_data = bpf.take_map("FILTER_MAP").expect("Missing FILTER_MAP");
    let filter_map: AyaHashMap<_, u32, RouteFilterPod> =
        AyaHashMap::try_from(filter_map_data).unwrap();

    // STATIC eBPF ATTACHMENT (Safely skips if interface doesn't exist yet)
    let overlay: &mut Xdp = bpf
        .program_mut("overlay_ingress")
        .unwrap()
        .try_into()
        .unwrap();
    overlay.load().unwrap();
    if get_ifindex(overlay_iface) > 0 {
        overlay.attach(overlay_iface, XdpFlags::SKB_MODE).unwrap();
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
        underlay.attach(underlay_iface, XdpFlags::SKB_MODE).unwrap();
        println!("Attached underlay_ingress to {}", underlay_iface);
    } else {
        println!("Warning: Underlay interface {} not found.", underlay_iface);
    }

    let mut state = GatewayState {
        routes: HashMap::new(),
        floating_ips: HashMap::new(),
        taps: HashMap::new(),
        crypto_keys: HashMap::new(),
        connections: HashMap::new(),
        filters: HashMap::new(),
        route_map,
        filter_map,
        fip_dnat_map,
        fip_snat_map,
        arp_proxy_map,
        bpf, // Retain Bpf context for dynamic API attachments
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
        gateway_ip: Some(gateway_ip),
        next_hop_ip: None,
        next_hop_mac: None,
        encrypted: false,
    };
    let target = build_route_target(&req, &state.taps).map_err(anyhow::Error::msg)?;
    state
        .route_map
        .insert(u32::from(Ipv4Addr::UNSPECIFIED), RouteTargetPod(target), 0)?;

    let route_uuid = Uuid::new_v4();
    state.routes.insert(
        route_uuid,
        Route {
            uuid: route_uuid,
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
        overlay.attach(uplink_iface, XdpFlags::SKB_MODE)?;
    }

    let mut uplinks: AyaHashMap<_, u32, ArpProxyPod> =
        AyaHashMap::try_from(state.bpf.map_mut("UPLINK_MAP").expect("Missing UPLINK_MAP"))?;
    let uplink = ArpProxy {
        mac: get_mac_address(uplink_iface),
        _pad: [0; 2],
        vm_ip: 0,
    };
    uplinks.insert(ifindex, ArpProxyPod(uplink), 0)?;

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
            gateway_ip: None,
            next_hop_ip: Some(next_hop),
            next_hop_mac: None,
            encrypted: false,
        };
        let target = build_route_target(&req, &state.taps).map_err(anyhow::Error::msg)?;
        state
            .route_map
            .insert(u32::from(dest_ip), RouteTargetPod(target), 0)?;

        let route_uuid = Uuid::new_v4();
        state.routes.insert(
            route_uuid,
            Route {
                uuid: route_uuid,
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
