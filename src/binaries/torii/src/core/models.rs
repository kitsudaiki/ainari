//! Data structures of the gateway API and of its internal bookkeeping.
//!
//! Everything in here is plain data: the payloads that go in and out of the HTTP
//! endpoints, the internal representations the gateway keeps for routes, TAP
//! devices and IPsec connections, plus the wrappers that make the eBPF map
//! values usable with Aya.

use std::net::Ipv4Addr;
use uuid::Uuid;

use torii_common::{ArpProxy, RouteFilter, RouteTarget};

/// Wrapper for passing RouteTarget to Aya eBPF maps safely.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RouteTargetPod(pub RouteTarget);

unsafe impl aya::Pod for RouteTargetPod {}

/// Wrapper for passing ArpProxy to Aya eBPF maps safely.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ArpProxyPod(pub ArpProxy);

unsafe impl aya::Pod for ArpProxyPod {}

/// Wrapper for passing RouteFilter to Aya eBPF maps safely.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RouteFilterPod(pub RouteFilter);

unsafe impl aya::Pod for RouteFilterPod {}

/// Bookkeeping for a TAP device managed by this gateway.
///
/// The gateway keeps the link layer details of every TAP around so that routes
/// pointing at the device can be programmed with the MAC of the VM behind it
/// without the control plane having to repeat that information per route. The
/// address of the VM is not kept here: it goes straight into the eBPF ARP
/// responder, the host route and the neighbour entry of the device.
#[derive(Debug, Clone)]
pub struct TapInfo {
    pub tap_mac: [u8; 6],
    pub vm_mac: Option<[u8; 6]>,
}

#[derive(Debug, Clone)]
pub struct CryptoKey {
    pub direction: String,
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub spi: u32,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub enabled: bool,
    pub active_egress_spi: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub uuid: Uuid,
    pub dest_ip: Ipv4Addr,
    pub target_iface: String,
    pub gateway_ip: Option<Ipv4Addr>,
    pub next_hop_ip: Option<Ipv4Addr>,
    pub next_hop_mac: Option<String>,
    pub encrypted: bool,
}
