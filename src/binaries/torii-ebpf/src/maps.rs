use aya_ebpf::macros::map;
use aya_ebpf::maps::{Array, HashMap};
use torii_common::{
    ArpProxy, CONFIG_ENTRIES, CONFIG_UPLINK_MODE, FipTarget, IfaceConfig, RouteFilter, RouteKey,
    RouteTarget, VNI_DEFAULT,
};

/// Routes of every tenant, keyed by `(vni, destination)`.
///
/// The tenant is part of the key because the destination alone stopped being
/// unique the moment two VMs of different tenants were allowed onto the same
/// gateway with the same address.
#[map]
pub static ROUTE_MAP: HashMap<RouteKey, RouteTarget> = HashMap::with_max_entries(1024, 0);

/// Interfaces (by ifindex) on which this gateway answers ARP requests itself.
///
/// Populated by the userspace control plane whenever a TAP device is created.
#[map]
pub static ARP_PROXY_MAP: HashMap<u32, ArpProxy> = HashMap::with_max_entries(1024, 0);

/// Packet filters of the routes, keyed by the very same key as `ROUTE_MAP`.
///
/// A route without an entry in this map is unfiltered. The key is the route key
/// a packet was matched to - not the address of the packet - so the filter of
/// the default route never leaks onto a destination that has a route of its own,
/// and the filter of one tenant never applies to another.
#[map]
pub static FILTER_MAP: HashMap<RouteKey, RouteFilter> = HashMap::with_max_entries(1024, 0);

/// Tenant and behaviour of every interface the overlay program is attached to.
///
/// This is the only place a packet entering from a VM can get its tenant from -
/// it is a property of the port it arrived on, never of anything the VM writes
/// into the packet.
#[map]
pub static IFACE_MAP: HashMap<u32, IfaceConfig> = HashMap::with_max_entries(1024, 0);

/// Floating IP to VM, keyed by the floating IP alone.
///
/// Floating IPs are unique across all tenants, so no VNI is needed to find one.
/// The tenant of the VM behind it travels in the value and becomes the tenant of
/// the packet.
#[map]
pub static FIP_DNAT_MAP: HashMap<u32, FipTarget> = HashMap::with_max_entries(1024, 0);

/// VM to floating IP, keyed by `(vni, internal address)`.
///
/// The reverse direction of `FIP_DNAT_MAP` needs the tenant in the key: the
/// internal address is exactly the thing that may repeat across tenants.
#[map]
pub static FIP_SNAT_MAP: HashMap<RouteKey, u32> = HashMap::with_max_entries(1024, 0);

/// Uplink interfaces (by ifindex) of the single gateway setup.
///
/// The value carries the MAC of the uplink, which is what the ARP responder
/// hands out for the floating IPs. Stays empty in the split setup.
#[map]
pub static UPLINK_MAP: HashMap<u32, ArpProxy> = HashMap::with_max_entries(16, 0);

/// Global switches of the datapath, indexed by the `CONFIG_*` constants.
///
/// An array map is zero-initialized, so a gateway that never writes into it
/// runs with every switch off - which is the split setup.
#[map]
pub static GATEWAY_CONFIG: Array<u32> = Array::with_max_entries(CONFIG_ENTRIES, 0);

/// Queries the routing map for a destination inside one tenant.
///
/// This function attempts to find an exact match for the given address in the
/// tenant the packet belongs to. If a specific route does not exist, it falls
/// back to the default route (`0.0.0.0`) of that same tenant, and finally to the
/// default route of the shared tenant.
///
/// Only the *default* route of the shared tenant is ever borrowed, never one of
/// its specific destinations. That route is the way out of the virtual network -
/// it is what the uplink of the edge gateway is programmed as at startup, from
/// the config, which knows nothing about tenants. Falling back to it therefore
/// lets a tenant reach the outside without letting it reach anything that lives
/// inside another tenant: an address of another tenant has no entry under the
/// shared tenant at all.
///
/// The route is borrowed, the packet is not: it keeps the tenant it came from.
/// A default route of the shared tenant is an encapsulating one on every gateway
/// that is not the edge itself, and the edge is where the floating IP of the
/// sender is put back in front of its internal address - a translation that is
/// keyed by `(vni, internal address)`. Stamping the shared tenant into the VXLAN
/// header here would hand the edge a packet whose sender it can no longer
/// resolve, so the target is handed back with the tenant of the packet.
///
/// # Arguments
/// * `vni` - The tenant the packet belongs to
/// * `ip` - The destination IPv4 address represented as a `u32`
///
/// # Returns
/// An `Option` holding the key the route was found under together with the
/// routing instruction, or `None` if no route matches. The key is what the
/// packet filter of the route is stored under, so it has to travel with the
/// target.
#[inline(always)]
pub fn lookup_route(vni: u32, ip: u32) -> Option<(RouteKey, RouteTarget)> {
    let key = RouteKey::new(vni, ip);
    if let Some(target) = unsafe { ROUTE_MAP.get(key) } {
        return Some((key, *target));
    }
    // Fallback to the default route (0.0.0.0) of this tenant
    let fallback = RouteKey::default_route(vni);
    if let Some(target) = unsafe { ROUTE_MAP.get(fallback) } {
        return Some((fallback, *target));
    }
    // ... and finally to the way out of the virtual network, which the shared
    // tenant owns.
    if vni != VNI_DEFAULT {
        let shared = RouteKey::default_route(VNI_DEFAULT);
        if let Some(target) = unsafe { ROUTE_MAP.get(shared) } {
            // The way out is shared, the packet is not: it travels on in its own
            // tenant, so the gateway at the other end can still tell whose it is.
            let mut borrowed = *target;
            borrowed.vni = vni;
            return Some((shared, borrowed));
        }
    }
    None
}

/// Queries the routing map for a destination without the default fallback.
///
/// Used by the ARP responder, which must be able to distinguish "we know an
/// explicit route for this address" from "the default route would swallow it".
///
/// # Arguments
/// * `vni` - The tenant the request arrived in
/// * `ip` - The destination IPv4 address represented as a `u32`
///
/// # Returns
/// An `Option<RouteTarget>` containing the exact routing entry, or `None`.
#[inline(always)]
pub fn lookup_route_exact(vni: u32, ip: u32) -> Option<RouteTarget> {
    unsafe { ROUTE_MAP.get(RouteKey::new(vni, ip)) }.copied()
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

/// Resolves the tenant and the behaviour of an ingress interface.
///
/// An interface the control plane never registered is treated as a port of the
/// default tenant that performs floating IP translation, which is byte for byte
/// how the datapath behaved before tenants existed.
///
/// # Arguments
/// * `ifindex` - The kernel interface index the packet was received on
///
/// # Returns
/// The `IfaceConfig` of the interface, or the unregistered default
#[inline(always)]
pub fn lookup_iface(ifindex: u32) -> IfaceConfig {
    match unsafe { IFACE_MAP.get(ifindex) } {
        Some(cfg) => *cfg,
        None => IfaceConfig::unregistered(),
    }
}

/// Looks up the packet filter attached to a route.
///
/// # Arguments
/// * `route_key` - The `(vni, destination)` key the route was matched under, as
///   returned by [`lookup_route`]
///
/// # Returns
/// A reference to the `RouteFilter` of the route, or `None` when the route is
/// unfiltered and therefore carries everything.
#[inline(always)]
pub fn lookup_filter(route_key: RouteKey) -> Option<&'static RouteFilter> {
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
