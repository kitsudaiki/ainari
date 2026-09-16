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

use aya::maps::HashMap as AyaHashMap;
use aya::programs::{Xdp, XdpFlags};
use aya::{Bpf, include_bytes_aligned};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::core::models::{ArpProxyPod, RouteFilterPod, RouteTargetPod};
use crate::core::state::GatewayState;
use crate::core::utils::{enable_forwarding, get_ifindex};

lazy_static::lazy_static! {
    pub static ref GATEWAY_STATE_HANDLE: Arc<Mutex<GatewayState>> = Arc::new(Mutex::new(init_routing()));
}

pub fn init_routing() -> GatewayState {
    let overlay_iface = std::env::var("OVERLAY_IFACE").unwrap_or_else(|_| "veth-gw".to_string());
    let underlay_iface = std::env::var("UNDERLAY_IFACE").unwrap_or_else(|_| "eth0".to_string());

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
    if get_ifindex(&overlay_iface) > 0 {
        overlay.attach(&overlay_iface, XdpFlags::SKB_MODE).unwrap();
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
    if get_ifindex(&underlay_iface) > 0 {
        underlay
            .attach(&underlay_iface, XdpFlags::SKB_MODE)
            .unwrap();
        println!("Attached underlay_ingress to {}", underlay_iface);
    } else {
        println!("Warning: Underlay interface {} not found.", underlay_iface);
    }

    GatewayState {
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
    }
}
