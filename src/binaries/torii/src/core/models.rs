//! Data structures of the gateway API and of its internal bookkeeping.
//!
//! Everything in here is plain data: the payloads that go in and out of the HTTP
//! endpoints, the internal representations the gateway keeps for routes, TAP
//! devices and IPsec connections, plus the wrappers that make the eBPF map
//! values usable with Aya.

use std::net::Ipv4Addr;
use uuid::Uuid;

use torii_common::{ArpProxy, FipTarget, IfaceConfig, RouteFilter, RouteKey, RouteTarget};

use ainari_api_structs::network_crypto_structs::CryptoDirection;

/// Wrapper for passing RouteKey to Aya eBPF maps safely.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct RouteKeyPod(pub RouteKey);

unsafe impl aya::Pod for RouteKeyPod {}

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

/// Wrapper for passing IfaceConfig to Aya eBPF maps safely.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct IfaceConfigPod(pub IfaceConfig);

unsafe impl aya::Pod for IfaceConfigPod {}

/// Wrapper for passing FipTarget to Aya eBPF maps safely.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct FipTargetPod(pub FipTarget);

unsafe impl aya::Pod for FipTargetPod {}

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
/// tenant is kept as well, because a route pointing at a TAP has to end up in
/// the same tenant as the port itself - otherwise the packet would be delivered
/// to a VM that is not supposed to see it.
///
/// The address of the VM is not kept here: it goes straight into the eBPF ARP
/// responder, the host route and the neighbour entry of the device.
#[derive(Debug, Clone)]
pub struct TapInfo {
    pub vni: u32,
    pub tap_mac: [u8; 6],
    pub vm_mac: Option<[u8; 6]>,
}

/// One floating IP together with the VM of the tenant it stands for.
///
/// A floating IP is unique across all tenants - it is the address the outside
/// world uses - so the gateway keys its bookkeeping by it alone and carries the
/// tenant of the internal address in the value.
#[derive(Debug, Clone)]
pub struct FloatingIp {
    pub vni: u32,
    pub internal_ip: Ipv4Addr,
}

#[derive(Debug, Clone)]
pub struct CryptoKey {
    pub direction: CryptoDirection,
    pub vni: u32,
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub spi: u32,
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub vni: u32,
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub enabled: bool,
    pub active_egress_spi: Option<u32>,
}

/// Internal representation of a network route.
///
/// `vni` and `dest_ip` together are what the eBPF maps of the datapath are keyed
/// by, so the same destination may exist once per tenant.
#[derive(Debug, Clone)]
pub struct Route {
    pub uuid: Uuid,
    pub vni: u32,
    pub dest_ip: Ipv4Addr,
    pub target_iface: String,
    pub gateway_ip: Option<Ipv4Addr>,
    pub next_hop_ip: Option<Ipv4Addr>,
    pub next_hop_mac: Option<String>,
    pub encrypted: bool,
}
