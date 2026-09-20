use crate::headers::{Ipv4Hdr, UdpHdr};
use crate::utils::{ipv4_checksum, ptr_at_mut};
use aya_ebpf::bindings::xdp_action;
use aya_ebpf::programs::XdpContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::IpProto;
use torii_common::RouteTarget;

/// Encapsulates a local packet in a UDP tunnel and forwards it over the underlay network.
///
/// Expands the packet buffer head by 42 bytes to accommodate the new Ethernet, IPv4, and UDP
/// headers. It constructs these headers using the target's underlay IP/MAC addresses
/// specified in the `RouteTarget` and recalculates checksums before issuing an XDP redirect.
///
/// # Arguments
/// * `ctx` - The XDP context for the packet being processed
/// * `target` - The routing instruction containing the underlay configuration (IPs and MACs)
///
/// # Returns
/// An eBPF `xdp_action` code, predominantly `XDP_REDIRECT` on success or `XDP_DROP` on failure
#[inline(always)]
pub fn encap_and_redirect(ctx: &XdpContext, target: &RouteTarget) -> u32 {
    if unsafe { aya_ebpf::helpers::bpf_xdp_adjust_head(ctx.ctx, -42) } != 0 {
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
        source: u16::to_be(5555),
        dest: u16::to_be(5555),
        len: u16::to_be(pkt_len - EthHdr::LEN as u16 - Ipv4Hdr::LEN as u16),
        check: 0,
    };
    unsafe { core::ptr::write_unaligned(new_udphdr, udp_hdr) };

    unsafe { aya_ebpf::helpers::bpf_redirect(target.ifindex, 0) as u32 }
}
