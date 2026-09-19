#![no_std]

/// Route action: deliver the packet out of a local interface.
pub const ROUTE_ACTION_LOCAL: u32 = 0;
/// Route action: wrap the packet into the L2-in-UDP overlay and send it to a
/// remote gateway over the underlay network.
pub const ROUTE_ACTION_ENCAP: u32 = 1;
/// Route action: hand the packet to the local kernel network stack.
///
/// Used for traffic that has to be protected by IPsec: the kernel owns the
/// xfrm state, so the packet must leave XDP and travel through the regular
/// forwarding path where the ESP transformation is applied. The eBPF overlay is
/// bypassed for those destinations - the kernel builds the tunnel instead.
pub const ROUTE_ACTION_KERNEL: u32 = 2;

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

/// Target descriptor used inside eBPF maps for routing logic.
///
/// This C-compatible struct provides the eBPF application with routing
/// directives. Depending on `action`, it directs the router to either
/// forward a packet locally (out of `ifindex`) or to perform UDP tunnel
/// encapsulation using the provided Underlay IP and MAC addresses.
///
/// For local forwarding (`action == 0`) the router behaves like a real L3 hop:
/// it rewrites the Ethernet header with `l2_src_mac`/`l2_dst_mac` before the
/// redirect. This is what allows the TAP devices to stay completely
/// unnumbered (no IP, no subnet) while still delivering frames with the
/// link-layer addresses the receiving VM (or the host) actually accepts.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct RouteTarget {
    pub action: u32,  // 0 = Local Interface, 1 = Encapsulate & Send, 2 = Kernel (IPsec)
    pub ifindex: u32, // Local interface index to redirect out of (e.g., eth0)
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
/// address is unambiguous and lets several VMs of the *same* subnet live on the
/// same host without any address collision on the host side.
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
