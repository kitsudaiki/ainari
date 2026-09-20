use crate::filter::filter_allows;
use crate::forward::redirect_local;
use crate::headers::{Ipv4Hdr, UdpHdr};
use crate::maps::{is_uplink, lookup_route, uplink_mode};
use crate::nat::{apply_snat, destination_ip};
use crate::utils::ptr_at;
use aya_ebpf::bindings::xdp_action;
use aya_ebpf::programs::XdpContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::IpProto;
use torii_common::ROUTE_ACTION_LOCAL;

/// Evaluates whether an incoming packet is a UDP tunnel packet.
///
/// Parses the Ethernet, IPv4, and UDP headers to determine if the packet matches
/// our overlay network criteria, specifically looking for our target destination port (5555).
///
/// # Arguments
/// * `ctx` - The XDP context containing packet data pointers
///
/// # Returns
/// A boolean indicating `true` if it's a tunnel packet, `false` otherwise
#[inline(always)]
pub fn is_tunnel_packet(ctx: &XdpContext) -> bool {
    let ethhdr = match ptr_at::<EthHdr>(ctx, 0) {
        Ok(hdr) => hdr,
        Err(_) => return false,
    };

    if unsafe { core::ptr::read_unaligned(ethhdr).ether_type } != EtherType::Ipv4 {
        return false;
    }

    let ipv4hdr = match ptr_at::<Ipv4Hdr>(ctx, EthHdr::LEN) {
        Ok(hdr) => hdr,
        Err(_) => return false,
    };

    if unsafe { core::ptr::read_unaligned(ipv4hdr).protocol } != IpProto::Udp as u8 {
        return false;
    }

    let udphdr = match ptr_at::<UdpHdr>(ctx, EthHdr::LEN + Ipv4Hdr::LEN) {
        Ok(hdr) => hdr,
        Err(_) => return false,
    };

    // FIX: Read into a variable first to avoid parser ambiguity
    let dest_port = unsafe { core::ptr::read_unaligned(udphdr).dest };
    dest_port == u16::to_be(5555)
}

/// Decapsulates a tunnel packet and routes its inner payload.
///
/// This function strips away the outer UDP/IPv4/Eth headers (42 bytes total), applies
/// SNAT translations to the inner payload if mapping exists, and finally queries the
/// eBPF routing map to redirect the packet to the correct local TAP interface.
///
/// The packet filter of the matched route is applied before the delivery, so an
/// include-list also guards what arrives from a remote gateway.
///
/// The inner Ethernet header still carries the addresses of the sending side, so
/// the local delivery rewrites it to the next hop of the target link before the
/// redirect. Without that step an unnumbered TAP would hand the VM a frame with
/// a foreign destination MAC, which the VM would silently discard.
///
/// In the split setup every decapsulated packet is masked behind its floating
/// IP, because the edge gateway only ever forwards it to the uplink. In the
/// single gateway setup (uplink mode) the SNAT is limited to packets that
/// really leave through an uplink, so tunnel traffic towards a local VM keeps
/// its source address.
///
/// # Arguments
/// * `ctx` - The XDP context containing the raw network packet
///
/// # Returns
/// An eBPF `xdp_action` code indicating the fate of the packet (e.g., `XDP_REDIRECT`, `XDP_DROP`)
#[inline(always)]
pub fn process_tunnel_packet(ctx: &XdpContext) -> u32 {
    // Strip Outer Eth, IPv4, and UDP headers (14 + 20 + 8 = 42 bytes)
    if unsafe { aya_ebpf::helpers::bpf_xdp_adjust_head(ctx.ctx, 42) } != 0 {
        return xdp_action::XDP_DROP;
    }

    // Parse the decapsulated Inner Ethernet frame
    let inner_eth = match ptr_at::<EthHdr>(ctx, 0) {
        Ok(hdr) => hdr,
        Err(_) => return xdp_action::XDP_DROP,
    };

    let eth_type = unsafe { core::ptr::read_unaligned(inner_eth).ether_type };

    let uplink_mode = uplink_mode();

    // Apply SNAT if necessary. Returns the true Target IP
    let dest_ip = if uplink_mode {
        destination_ip(ctx, eth_type)
    } else {
        apply_snat(ctx, eth_type)
    };

    if let Some(dest_ip) = dest_ip
        && let Some((route_key, target)) = lookup_route(dest_ip)
        && target.action == ROUTE_ACTION_LOCAL
    {
        // Enforce the include-lists of the route on the receiving side as well:
        // a packet that entered the overlay on another host has not been seen by
        // this filter yet.
        if !filter_allows(ctx, eth_type, route_key) {
            return xdp_action::XDP_DROP;
        }
        if uplink_mode && is_uplink(target.ifindex) {
            apply_snat(ctx, eth_type);
        }
        return redirect_local(ctx, &target);
    }

    // Drop invalid tunnel packets
    xdp_action::XDP_DROP
}
