//! Runtime state shared by all HTTP handlers.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use aya::Bpf;
use aya::maps::{HashMap as AyaHashMap, MapData};
use uuid::Uuid;

use crate::core::models::{ArpProxyPod, RouteFilterPod, RouteTargetPod, TapInfo, Route, Connection, CryptoKey};

use ainari_api_structs::network_filter_structs::*;

/// Holds the application's runtime state, mapped variables, and eBPF context.
pub struct GatewayState {
    pub routes: HashMap<Uuid, Route>,
    pub floating_ips: HashMap<Ipv4Addr, Ipv4Addr>,
    pub taps: HashMap<String, TapInfo>, // TAP devices and the VMs behind them
    pub crypto_keys: HashMap<String, CryptoKey>, // installed IPsec keys, by "direction:spi"
    pub connections: HashMap<String, Connection>, // VM-to-VM connections, by "local->remote"
    pub filters: HashMap<Uuid, RouteFilterRules>, // packet filters, by route uuid
    pub route_map: AyaHashMap<MapData, u32, RouteTargetPod>,
    pub filter_map: AyaHashMap<MapData, u32, RouteFilterPod>,
    pub fip_dnat_map: AyaHashMap<MapData, u32, u32>,
    pub fip_snat_map: AyaHashMap<MapData, u32, u32>,
    pub arp_proxy_map: AyaHashMap<MapData, u32, ArpProxyPod>,
    // Kept in state both to attach programs to TAP devices created later on and
    // because dropping it would detach the running XDP programs.
    pub bpf: Bpf,
}
