use crate::headers::{Ipv4Hdr, UdpHdr, VxlanHdr};
use crate::utils::{ipv4_checksum, ptr_at_mut};
use aya_ebpf::bindings::xdp_action;
use aya_ebpf::programs::XdpContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::IpProto;
use torii_common::{OVERLAY_OVERHEAD, OVERLAY_PORT, RouteTarget};

/// The overhead the overlay claims has to be exactly the headers it writes,
/// otherwise encapsulation and decapsulation would disagree about where the
/// inner frame starts.
const _: () = assert!(OVERLAY_OVERHEAD == EthHdr::LEN + Ipv4Hdr::LEN + UdpHdr::LEN + VxlanHdr::LEN);

/// Encapsulates a local packet in a VXLAN tunnel and forwards it over the underlay network.
///
/// Expands the packet buffer head by [`OVERLAY_OVERHEAD`] bytes to accommodate the new
/// Ethernet, IPv4, UDP and VXLAN headers. It constructs these headers using the target's
/// underlay IP/MAC addresses specified in the `RouteTarget` and recalculates checksums
/// before issuing an XDP redirect.
///
/// The VXLAN header carries the tenant of the route. That is the whole point of
/// the outer headers here: the receiving gateway has no other way to tell which
/// of its tenants an inner address belongs to, because the very same address may
/// exist in several of them.
///
/// # Arguments
/// * `ctx` - The XDP context for the packet being processed
/// * `target` - The routing instruction containing the underlay configuration (IPs, MACs, VNI)
///
/// # Returns
/// An eBPF `xdp_action` code, predominantly `XDP_REDIRECT` on success or `XDP_DROP` on failure
#[inline(always)]
pub fn encap_and_redirect(ctx: &XdpContext, target: &RouteTarget) -> u32 {
    if unsafe { aya_ebpf::helpers::bpf_xdp_adjust_head(ctx.ctx, -(OVERLAY_OVERHEAD as i32)) } != 0 {
        return xdp_action::XDP_DROP;
    }

    let pkt_len = (ctx.data_end() - ctx.data()) as u16;

    let new_ethhdr = match ptr_at_mut::<EthHdr>(ctx, 0) {
        Ok(h) => h,
        Err(_) => return xdp_action::XDP_DROP,
    };
    let new_ipv4hdr = match ptr_at_mut::<Ipv4Hdr>(ctx, EthHdr::LEN) {
        Ok(h) => h,
        Err(_) => return xdp_action::XDP_DROP,
    };
    let new_udphdr = match ptr_at_mut::<UdpHdr>(ctx, EthHdr::LEN + Ipv4Hdr::LEN) {
        Ok(h) => h,
        Err(_) => return xdp_action::XDP_DROP,
    };
    let new_vxlanhdr = match ptr_at_mut::<VxlanHdr>(ctx, EthHdr::LEN + Ipv4Hdr::LEN + UdpHdr::LEN) {
        Ok(h) => h,
        Err(_) => return xdp_action::XDP_DROP,
    };

    let mut eth = unsafe { core::ptr::read_unaligned(new_ethhdr) };
    eth.src_addr = target.encap_src_mac;
    eth.dst_addr = target.encap_dst_mac;
    eth.ether_type = EtherType::Ipv4;
    unsafe { core::ptr::write_unaligned(new_ethhdr, eth) };

    let mut ip_hdr = Ipv4Hdr {
        version_ihl: (4 << 4) | 5,
        tos: 0,
        tot_len: u16::to_be(pkt_len - EthHdr::LEN as u16),
        id: 0,
        frag_off: 0,
        ttl: 64,
        protocol: IpProto::Udp as u8,
        check: 0,
        src_addr: u32::to_be(target.encap_src_ip),
        dst_addr: u32::to_be(target.encap_dst_ip),
    };
    ip_hdr.check = ipv4_checksum(&ip_hdr);
    unsafe { core::ptr::write_unaligned(new_ipv4hdr, ip_hdr) };

    let udp_hdr = UdpHdr {
        source: u16::to_be(OVERLAY_PORT),
        dest: u16::to_be(OVERLAY_PORT),
        len: u16::to_be(pkt_len - EthHdr::LEN as u16 - Ipv4Hdr::LEN as u16),
        check: 0,
    };
    unsafe { core::ptr::write_unaligned(new_udphdr, udp_hdr) };

    // The tenant the inner frame belongs to, so the far side can look it up in
    // the right half of its routing map.
    unsafe { core::ptr::write_unaligned(new_vxlanhdr, VxlanHdr::new(target.vni)) };

    unsafe { aya_ebpf::helpers::bpf_redirect(target.ifindex, 0) as u32 }
}
