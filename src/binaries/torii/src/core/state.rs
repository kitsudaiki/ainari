//! Runtime state shared by all HTTP handlers.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use aya::Ebpf;
use aya::maps::{HashMap as AyaHashMap, MapData};
use uuid::Uuid;

use crate::core::models::{
    ArpProxyPod, Connection, CryptoKey, FipTargetPod, FloatingIp, IfaceConfigPod, Route,
    RouteFilterPod, RouteKeyPod, RouteTargetPod, TapInfo,
};

use ainari_api_structs::network_crypto_structs::CryptoDirection;
use ainari_api_structs::network_filter_structs::*;

/// Holds the application's runtime state, mapped variables, and eBPF context.
///
/// The two maps that decide where a packet goes - `route_map` and `filter_map` -
/// are keyed by `(vni, destination)`, so the same address can be present once
/// per tenant. `fip_dnat_map` is the one map keyed by a bare address, because a
/// floating IP is unique by definition; its value carries the tenant instead.
pub struct GatewayState {
    pub routes: HashMap<Uuid, Route>,
    pub floating_ips: HashMap<Ipv4Addr, FloatingIp>,
    pub taps: HashMap<String, TapInfo>, // TAP devices and the VMs behind them
    pub crypto_keys: HashMap<(CryptoDirection, u32), CryptoKey>, // installed IPsec keys, by direction and spi
    pub connections: HashMap<String, Connection>, // VM-to-VM connections, by "vni:local->remote"
    pub filters: HashMap<Uuid, RouteFilterRules>, // packet filters, by route uuid
    pub route_map: AyaHashMap<MapData, RouteKeyPod, RouteTargetPod>,
    pub filter_map: AyaHashMap<MapData, RouteKeyPod, RouteFilterPod>,
    pub fip_dnat_map: AyaHashMap<MapData, u32, FipTargetPod>,
    pub fip_snat_map: AyaHashMap<MapData, RouteKeyPod, u32>,
    pub arp_proxy_map: AyaHashMap<MapData, u32, ArpProxyPod>,
    pub iface_map: AyaHashMap<MapData, u32, IfaceConfigPod>,
    // Kept in state both to attach programs to TAP devices created later on and
    // because dropping it would detach the running XDP programs.
    pub bpf: Ebpf,
}
