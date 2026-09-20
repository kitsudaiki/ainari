use crate::utils::ptr_at_mut;
use aya_ebpf::bindings::xdp_action;
use aya_ebpf::programs::XdpContext;
use network_types::eth::EthHdr;
use torii_common::RouteTarget;

/// Checks whether a MAC address slot of a route is unconfigured.
///
/// An all-zero MAC is used by the control plane to express "no link layer
/// information available", in which case the frame is forwarded untouched.
///
/// # Arguments
/// * `mac` - The MAC address taken from a `RouteTarget`
///
/// # Returns
/// `true` if every byte of the address is zero
#[inline(always)]
fn mac_unset(mac: &[u8; 6]) -> bool {
    (mac[0] | mac[1] | mac[2] | mac[3] | mac[4] | mac[5]) == 0
}

/// Delivers a packet to a local interface, acting as a proper L3 next hop.
///
/// Since the TAP devices are unnumbered and the VMs learn a single gateway MAC
/// via the eBPF ARP responder, every frame arrives addressed to that gateway
/// MAC. Before handing the frame to its final interface the Ethernet header is
/// therefore rewritten: the source becomes the outgoing interface (the router
/// port) and the destination becomes the next hop on that link - the MAC of the
/// VM behind a TAP, or the MAC of the host NIC behind the veth uplink.
///
/// Routes without link layer information (all-zero MACs) are redirected
/// unmodified, which keeps plain L2 overlay forwarding working.
///
/// # Arguments
/// * `ctx` - The XDP context of the packet being forwarded
/// * `target` - The matched routing entry describing the local interface
///
/// # Returns
/// An eBPF `xdp_action` code, normally `XDP_REDIRECT`, or `XDP_DROP` on error
#[inline(always)]
pub fn redirect_local(ctx: &XdpContext, target: &RouteTarget) -> u32 {
    if !mac_unset(&target.l2_dst_mac) {
        let eth_ptr = match ptr_at_mut::<EthHdr>(ctx, 0) {
            Ok(ptr) => ptr,
            Err(_) => return xdp_action::XDP_DROP,
        };

        let mut eth = unsafe { core::ptr::read_unaligned(eth_ptr) };
        eth.dst_addr = target.l2_dst_mac;
        if !mac_unset(&target.l2_src_mac) {
            eth.src_addr = target.l2_src_mac;
        }
        unsafe { core::ptr::write_unaligned(eth_ptr, eth) };
    }

    unsafe { aya_ebpf::helpers::bpf_redirect(target.ifindex, 0) as u32 }
}
