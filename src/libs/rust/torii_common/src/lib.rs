#![no_std]

/// Route action: deliver the packet out of a local interface.
pub const ROUTE_ACTION_LOCAL: u32 = 0;
/// Route action: wrap the packet into the VXLAN overlay and send it to a
/// remote gateway over the underlay network.
pub const ROUTE_ACTION_ENCAP: u32 = 1;
/// Route action: hand the packet to the local kernel network stack.
///
/// Used for traffic that has to be protected by IPsec: the kernel owns the
/// xfrm state, so the packet must leave XDP and travel through the regular
/// forwarding path where the ESP transformation is applied. The eBPF overlay is
/// bypassed for those destinations - the kernel builds the tunnel instead.
pub const ROUTE_ACTION_KERNEL: u32 = 2;

/// The tenant every address lives in unless something says otherwise.
///
/// VNI 0 is the shared, untenanted space: it is what an interface that was
/// never registered belongs to, what the uplink towards the outside world uses
/// and what a setup that does not care about tenants ends up running in
/// entirely. Isolated tenants start at 1.
pub const VNI_DEFAULT: u32 = 0;

/// Largest VNI that still fits into the 24 bit field of a VXLAN header.
pub const VNI_MAX: u32 = 0x00ff_ffff;

/// UDP port the overlay tunnel is carried on.
pub const OVERLAY_PORT: u16 = 5555;

/// Bytes the overlay adds to a frame: Ethernet + IPv4 + UDP + VXLAN.
///
/// This is what the underlay MTU has to accommodate on top of the MTU the VMs
/// are configured with.
pub const OVERLAY_OVERHEAD: usize = 14 + 20 + 8 + 8;

/// Index of the uplink mode switch inside the `GATEWAY_CONFIG` array map.
///
/// `0` (the default) is the split setup: an edge gateway translates floating IPs
/// on every interface and applies the SNAT when it decapsulates the traffic that
/// the VMM gateways tunnel to it. `1` is the single gateway setup, in which the
/// VMs and the uplink live behind the same gateway: floating IPs are translated
/// only on the interfaces listed in `UPLINK_MAP`, and the SNAT is applied to
/// every packet that leaves through one of them.
pub const CONFIG_UPLINK_MODE: u32 = 0;
/// Number of entries of the `GATEWAY_CONFIG` array map.
pub const CONFIG_ENTRIES: u32 = 1;

/// Key of `ROUTE_MAP`, `FILTER_MAP` and `FIP_SNAT_MAP`: an address inside a tenant.
///
/// A plain destination address is not enough to decide where a packet goes as
/// soon as two VMs on the same gateway may carry the same address. The tenant
/// the packet belongs to is therefore part of every lookup, and the two halves
/// together are what a route, a filter and a floating IP are stored under.
///
/// Both fields are in host byte order.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct RouteKey {
    pub vni: u32,
    pub dst: u32,
}

impl RouteKey {
    /// Builds a key from a tenant and an address.
    ///
    /// # Arguments
    /// * `vni` - The tenant the address is valid in
    /// * `dst` - The IPv4 address in host byte order
    ///
    /// # Returns
    /// The `RouteKey` the eBPF maps are indexed with
    #[inline(always)]
    pub const fn new(vni: u32, dst: u32) -> Self {
        Self { vni, dst }
    }

    /// Builds the key of the default route of a tenant.
    ///
    /// # Arguments
    /// * `vni` - The tenant whose default route is wanted
    ///
    /// # Returns
    /// The key `0.0.0.0` is stored under inside that tenant
    #[inline(always)]
    pub const fn default_route(vni: u32) -> Self {
        Self { vni, dst: 0 }
    }
}

/// Target descriptor used inside eBPF maps for routing logic.
///
/// This C-compatible struct provides the eBPF application with routing
/// directives. Depending on `action`, it directs the router to either
/// forward a packet locally (out of `ifindex`) or to perform VXLAN
/// encapsulation using the provided Underlay IP and MAC addresses.
///
/// For local forwarding (`action == 0`) the router behaves like a real L3 hop:
/// it rewrites the Ethernet header with `l2_src_mac`/`l2_dst_mac` before the
/// redirect. This is what allows the TAP devices to stay completely
/// unnumbered (no IP, no subnet) while still delivering frames with the
/// link-layer addresses the receiving VM (or the host) actually accepts.
///
/// `vni` is the tenant the packet belongs to. For an encapsulated route it is
/// written into the VXLAN header, which is what lets the receiving gateway put
/// the packet back into the right tenant before it looks up anything.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct RouteTarget {
    pub action: u32,  // 0 = Local Interface, 1 = Encapsulate & Send, 2 = Kernel (IPsec)
    pub ifindex: u32, // Local interface index to redirect out of (e.g., eth0)
    pub vni: u32,     // Tenant of the route; goes into the VXLAN header on encap
    pub encap_dst_ip: u32, // Target Gateway Underlay IP
    pub encap_dst_mac: [u8; 6],
    pub _pad1: [u8; 2],
    pub encap_src_ip: u32, // Our Gateway Underlay IP
    pub encap_src_mac: [u8; 6],
    pub _pad2: [u8; 2],
    pub l2_dst_mac: [u8; 6], // Next-hop MAC for local delivery (VM or host NIC)
    pub _pad3: [u8; 2],
    pub l2_src_mac: [u8; 6], // MAC of the outgoing local interface (the router port)
    pub _pad4: [u8; 2],
}

/// Per-interface configuration for the eBPF ARP responder.
///
/// TAP devices are unnumbered, so the kernel can never answer the ARP requests
/// a VM emits for its default gateway or for its subnet neighbours. Instead the
/// XDP program answers them itself (proxy ARP) using `mac` - the MAC of the TAP
/// device the VM is attached to. Because every TAP is a point-to-point link
/// towards exactly one VM, answering with a single MAC for every queried
/// address is unambiguous and lets several VMs of the *same* subnet - and, with
/// tenants, of the *same address* - live on the same host without any collision
/// on the host side.
///
/// The same struct describes an uplink in the single gateway setup: there `mac`
/// is the MAC of the uplink, handed out for the floating IPs, and `vm_ip` is
/// unused (0).
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct ArpProxy {
    pub mac: [u8; 6], // MAC handed out in the ARP replies (the TAP's own MAC)
    pub _pad: [u8; 2],
    pub vm_ip: u32, // IP of the attached VM; never answered (0 = unknown)
}

/// `IfaceConfig` flag: floating IP translation happens on this interface.
///
/// Only a port that faces the outside world carries it. A floating IP is the
/// globally unique name of a VM, so the mapping is also what tells the datapath
/// which tenant an arriving packet belongs to - and that must not be possible
/// from inside a tenant, or a VM could reach another tenant by addressing its
/// floating IP.
pub const IFACE_FLAG_FIP: u32 = 1 << 0;

/// Per-interface configuration of the overlay datapath.
///
/// Every interface the `overlay_ingress` program is attached to belongs to
/// exactly one tenant, and that is where the VNI of a packet coming out of a VM
/// is taken from - the VM never gets to name its own tenant. An interface that
/// was never registered behaves like a port of the default tenant with floating
/// IP translation enabled, which is exactly how the datapath worked before
/// tenants existed.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct IfaceConfig {
    pub vni: u32,
    pub flags: u32,
}

impl IfaceConfig {
    /// Configuration assumed for an interface the control plane never registered.
    ///
    /// # Arguments
    /// None
    ///
    /// # Returns
    /// The default tenant with floating IP translation switched on
    #[inline(always)]
    pub const fn unregistered() -> Self {
        Self {
            vni: VNI_DEFAULT,
            flags: IFACE_FLAG_FIP,
        }
    }

    /// Reports whether floating IP translation runs on this interface.
    ///
    /// # Arguments
    /// None
    ///
    /// # Returns
    /// `true` when the interface carries `IFACE_FLAG_FIP`
    #[inline(always)]
    pub const fn does_fip(&self) -> bool {
        self.flags & IFACE_FLAG_FIP != 0
    }
}

/// Value of `FIP_DNAT_MAP`: the VM a floating IP stands for.
///
/// Floating IPs are unique across the whole setup - they are the addresses the
/// outside world uses - so the map is keyed by the floating IP alone. The
/// tenant travels in the value instead, because resolving a floating IP is
/// precisely the step that moves a packet from the shared uplink into the
/// tenant of the VM behind it.
///
/// Both fields are in host byte order.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct FipTarget {
    pub vni: u32,
    pub ip: u32,
}

/// Maximum number of IP ranges one route filter can hold.
///
/// The include-lists live inside a single eBPF map value, so their capacity has
/// to be fixed at compile time. Sixteen entries per list keep the value small
/// enough to stay cheap to look up while still allowing a handful of subnets
/// per route.
pub const FILTER_MAX_IP_RANGES: usize = 16;

/// Maximum number of port ranges one route filter can hold.
pub const FILTER_MAX_PORT_RANGES: usize = 16;

/// One entry of the IP include-list, stored as an inclusive range.
///
/// Subnets are converted into their first and last address by the control
/// plane, so a single representation covers both a CIDR (`10.0.0.0/24`) and an
/// explicit range (`10.0.0.5-10.0.0.9`). Both bounds are in host byte order.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct IpRange {
    pub start: u32,
    pub end: u32,
}

/// One entry of the port include-list, stored as an inclusive range.
///
/// A single port is expressed as a range whose bounds are equal. Both bounds
/// are in host byte order.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PortRange {
    pub start: u16,
    pub end: u16,
}

/// Packet filter of one route, as consumed by the eBPF datapath.
///
/// The filter is a pair of include-lists that is attached to the route a packet
/// was matched to. Both lists work the same way:
///
/// * an **empty** list allows everything - a route without IP ranges accepts
///   every address, a route without ports accepts every port
/// * a **non-empty** list allows only what it names
///
/// The lists are matched against different parts of the packet. `ip_ranges`
/// checks the *source* address: the destination is already pinned by the route
/// key itself, so the only open question is who is allowed to use the route.
/// `port_ranges` checks the source *and* the destination port and accepts the
/// packet when either of them is listed, which is what keeps the answers of an
/// allowed service flowing back through the reverse route of a stateless
/// filter.
///
/// Only the entries below `ip_range_count` / `port_range_count` are populated;
/// the rest of the arrays is padding that the datapath never reads.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct RouteFilter {
    pub ip_range_count: u32,
    pub port_range_count: u32,
    pub ip_ranges: [IpRange; FILTER_MAX_IP_RANGES],
    pub port_ranges: [PortRange; FILTER_MAX_PORT_RANGES],
}

impl RouteFilter {
    /// Builds a filter with both include-lists empty, i.e. one that allows
    /// every packet of its route.
    ///
    /// # Arguments
    /// None
    ///
    /// # Returns
    /// A `RouteFilter` with both counters set to zero
    pub const fn empty() -> Self {
        Self {
            ip_range_count: 0,
            port_range_count: 0,
            ip_ranges: [IpRange { start: 0, end: 0 }; FILTER_MAX_IP_RANGES],
            port_ranges: [PortRange { start: 0, end: 0 }; FILTER_MAX_PORT_RANGES],
        }
    }
}

impl Default for RouteFilter {
    fn default() -> Self {
        Self::empty()
    }
}
