use aya_ebpf::macros::map;
use aya_ebpf::maps::{Array, HashMap};
use torii_common::{ArpProxy, CONFIG_ENTRIES, CONFIG_UPLINK_MODE, RouteFilter, RouteTarget};

#[map]
pub static ROUTE_MAP: HashMap<u32, RouteTarget> = HashMap::with_max_entries(1024, 0);

/// Interfaces (by ifindex) on which this gateway answers ARP requests itself.
///
/// Populated by the userspace control plane whenever a TAP device is created.
#[map]
pub static ARP_PROXY_MAP: HashMap<u32, ArpProxy> = HashMap::with_max_entries(1024, 0);

/// Packet filters of the routes, keyed by the very same key as `ROUTE_MAP`.
///
/// A route without an entry in this map is unfiltered. The key is the route key
/// a packet was matched to - not the address of the packet - so the filter of
/// the default route never leaks onto a destination that has a route of its own.
#[map]
pub static FILTER_MAP: HashMap<u32, RouteFilter> = HashMap::with_max_entries(1024, 0);

#[map]
pub static FIP_DNAT_MAP: HashMap<u32, u32> = HashMap::with_max_entries(1024, 0);

#[map]
pub static FIP_SNAT_MAP: HashMap<u32, u32> = HashMap::with_max_entries(1024, 0);

/// Uplink interfaces (by ifindex) of the single gateway setup.
///
/// The value carries the MAC of the uplink, which is what the ARP responder
/// hands out for the floating IPs. Stays empty in the split setup.
#[map]
pub static UPLINK_MAP: HashMap<u32, ArpProxy> = HashMap::with_max_entries(16, 0);

/// Global switches of the datapath, indexed by the `CONFIG_*` constants.
///
/// An array map is zero-initialised, so a gateway that never writes into it
/// runs with every switch off - which is the split setup.
#[map]
pub static GATEWAY_CONFIG: Array<u32> = Array::with_max_entries(CONFIG_ENTRIES, 0);

/// Queries the routing map for a target IP address.
///
/// This function attempts to find an exact match for the given IP address in the
/// `ROUTE_MAP`. If a specific route does not exist, it falls back to the default
/// route (0.0.0.0).
///
/// # Arguments
/// * `ip` - The destination IPv4 address represented as a `u32`
///
/// # Returns
/// An `Option` holding the key the route was found under together with the
/// routing instruction, or `None` if no route matches. The key is what the
/// packet filter of the route is stored under, so it has to travel with the
/// target.
#[inline(always)]
pub fn lookup_route(ip: u32) -> Option<(u32, RouteTarget)> {
    if let Some(target) = unsafe { ROUTE_MAP.get(ip) } {
        return Some((ip, *target));
    }
    // Fallback to default route (0.0.0.0)
    if let Some(target) = unsafe { ROUTE_MAP.get(0) } {
        return Some((0, *target));
    }
    None
}

/// Queries the routing map for a target IP address without the default fallback.
///
/// Used by the ARP responder, which must be able to distinguish "we know an
/// explicit route for this address" from "the default route would swallow it".
///
/// # Arguments
/// * `ip` - The destination IPv4 address represented as a `u32`
///
/// # Returns
/// An `Option<RouteTarget>` containing the exact routing entry, or `None`.
#[inline(always)]
pub fn lookup_route_exact(ip: u32) -> Option<RouteTarget> {
    unsafe { ROUTE_MAP.get(ip) }.copied()
}

/// Looks up the ARP responder configuration of an ingress interface.
///
/// # Arguments
/// * `ifindex` - The kernel interface index the packet was received on
///
/// # Returns
/// An `Option<ArpProxy>` holding the MAC to answer with, or `None` if the
/// interface is not managed by the eBPF ARP responder (e.g. the underlay).
#[inline(always)]
pub fn lookup_arp_proxy(ifindex: u32) -> Option<ArpProxy> {
    unsafe { ARP_PROXY_MAP.get(ifindex) }.copied()
}

/// Looks up the packet filter attached to a route.
///
/// # Arguments
/// * `route_key` - The key the route was matched under, as returned by
///   [`lookup_route`]
///
/// # Returns
/// A reference to the `RouteFilter` of the route, or `None` when the route is
/// unfiltered and therefore carries everything.
#[inline(always)]
pub fn lookup_filter(route_key: u32) -> Option<&'static RouteFilter> {
    unsafe { FILTER_MAP.get(route_key) }
}

/// Tells whether the gateway runs in the single gateway (uplink) mode.
///
/// # Returns
/// `true` once the control plane has registered an uplink
#[inline(always)]
pub fn uplink_mode() -> bool {
    matches!(GATEWAY_CONFIG.get(CONFIG_UPLINK_MODE), Some(&mode) if mode != 0)
}

/// Looks up the uplink configuration of an interface.
///
/// # Arguments
/// * `ifindex` - The kernel interface index to check
///
/// # Returns
/// An `Option<ArpProxy>` holding the MAC of the uplink, or `None` if the
/// interface is no uplink (always the case in the split setup)
#[inline(always)]
pub fn lookup_uplink(ifindex: u32) -> Option<ArpProxy> {
    unsafe { UPLINK_MAP.get(ifindex) }.copied()
}

/// Checks whether an interface is an uplink of the single gateway setup.
///
/// # Arguments
/// * `ifindex` - The kernel interface index to check
///
/// # Returns
/// `true` if the interface is registered in `UPLINK_MAP`
#[inline(always)]
pub fn is_uplink(ifindex: u32) -> bool {
    unsafe { UPLINK_MAP.get(ifindex) }.is_some()
}

/// Checks whether an address is a floating IP served by this gateway.
///
/// # Arguments
/// * `ip` - The IPv4 address represented as a `u32`
///
/// # Returns
/// `true` if the address has an entry in `FIP_DNAT_MAP`
#[inline(always)]
pub fn is_floating_ip(ip: u32) -> bool {
    unsafe { FIP_DNAT_MAP.get(ip) }.is_some()
}
