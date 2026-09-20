use aya_ebpf::programs::XdpContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::ip::IpProto;
use torii_common::{FILTER_MAX_IP_RANGES, FILTER_MAX_PORT_RANGES, RouteFilter};

use crate::headers::{Ipv4Hdr, TcpHdr, UdpHdr};
use crate::maps::lookup_filter;
use crate::utils::ptr_at;

/// Checks whether a source address is named by the IP include-list.
///
/// An empty list means "no restriction", which is the state every route starts
/// in. As soon as one range is added the list becomes exclusive: only addresses
/// inside one of its ranges are carried by the route.
///
/// # Arguments
/// * `filter` - The filter of the route the packet was matched to
/// * `src_ip` - Source address of the packet in host byte order
///
/// # Returns
/// `true` when the list is empty or one of its ranges contains the address
#[inline(always)]
fn ip_allowed(filter: &RouteFilter, src_ip: u32) -> bool {
    if filter.ip_range_count == 0 {
        return true;
    }

    for i in 0..FILTER_MAX_IP_RANGES {
        if i as u32 >= filter.ip_range_count {
            break;
        }
        let range = filter.ip_ranges[i];
        if src_ip >= range.start && src_ip <= range.end {
            return true;
        }
    }

    false
}

/// Checks whether one of the ports of a packet is named by the port include-list.
///
/// Both ports are tested and either one is enough. A stateless filter sees the
/// answers of a service as well as its requests, and those carry the allowed
/// port as *source* port - demanding it in the destination would let the request
/// through and drop the reply.
///
/// # Arguments
/// * `filter` - The filter of the route the packet was matched to
/// * `src_port` - Source port of the packet in host byte order
/// * `dst_port` - Destination port of the packet in host byte order
///
/// # Returns
/// `true` when the list is empty or one of its ranges contains one of the ports
#[inline(always)]
fn port_allowed(filter: &RouteFilter, src_port: u16, dst_port: u16) -> bool {
    if filter.port_range_count == 0 {
        return true;
    }

    for i in 0..FILTER_MAX_PORT_RANGES {
        if i as u32 >= filter.port_range_count {
            break;
        }
        let range = filter.port_ranges[i];
        if (src_port >= range.start && src_port <= range.end)
            || (dst_port >= range.start && dst_port <= range.end)
        {
            return true;
        }
    }

    false
}

/// Reads the port pair of a packet, if the protocol carries one.
///
/// # Arguments
/// * `ctx` - The XDP context of the packet
/// * `protocol` - The IP protocol number taken from the IPv4 header
/// * `frag_off` - The fragment field of the IPv4 header, in network byte order
/// * `l4_offset` - Offset of the transport header inside the frame
///
/// # Returns
/// An `Option` with the source and destination port in host byte order, or
/// `None` for protocols without ports (ICMP, ESP, ...), for the follow-up
/// fragments of a fragmented datagram, which carry payload where the transport
/// header would be, and for truncated packets
#[inline(always)]
fn read_ports(
    ctx: &XdpContext,
    protocol: u8,
    frag_off: u16,
    l4_offset: usize,
) -> Option<(u16, u16)> {
    // Only the first fragment of a datagram holds the transport header.
    if u16::from_be(frag_off) & 0x1fff != 0 {
        return None;
    }

    if protocol == IpProto::Tcp as u8 {
        let tcp = ptr_at::<TcpHdr>(ctx, l4_offset).ok()?;
        let hdr = unsafe { core::ptr::read_unaligned(tcp) };
        Some((u16::from_be(hdr.source), u16::from_be(hdr.dest)))
    } else if protocol == IpProto::Udp as u8 {
        let udp = ptr_at::<UdpHdr>(ctx, l4_offset).ok()?;
        let hdr = unsafe { core::ptr::read_unaligned(udp) };
        Some((u16::from_be(hdr.source), u16::from_be(hdr.dest)))
    } else {
        None
    }
}

/// Applies the packet filter of a route to a packet that matched it.
///
/// The check runs directly before the forwarding decision is carried out, so it
/// sees the packet the way it will leave the gateway - floating IP translation
/// has already happened at that point.
///
/// Three things pass unconditionally:
///
/// * routes without a filter, which is every route until the control plane
///   attaches one
/// * everything that is not IPv4, i.e. the ARP traffic the datapath needs to
///   keep the unnumbered links alive
/// * protocols without ports (ICMP and friends) with respect to the *port*
///   list; they are still matched against the IP list
///
/// A packet whose IPv4 header cannot be parsed while a filter is installed is
/// rejected: a filter that cannot be evaluated must not turn into a hole.
///
/// # Arguments
/// * `ctx` - The XDP context of the packet
/// * `eth_type` - The already parsed EtherType of the frame
/// * `route_key` - The key the route was matched under
///
/// # Returns
/// `true` when the packet may be forwarded, `false` when it has to be dropped
#[inline(always)]
pub fn filter_allows(ctx: &XdpContext, eth_type: EtherType, route_key: u32) -> bool {
    if eth_type != EtherType::Ipv4 {
        return true;
    }

    let filter = match lookup_filter(route_key) {
        Some(filter) => filter,
        None => return true,
    };

    let ipv4 = match ptr_at::<Ipv4Hdr>(ctx, EthHdr::LEN) {
        Ok(ptr) => unsafe { core::ptr::read_unaligned(ptr) },
        Err(_) => return false,
    };

    if !ip_allowed(filter, u32::from_be(ipv4.src_addr)) {
        return false;
    }

    if filter.port_range_count == 0 {
        return true;
    }

    let l4_offset = EthHdr::LEN + ((ipv4.version_ihl & 0x0F) * 4) as usize;
    match read_ports(ctx, ipv4.protocol, ipv4.frag_off, l4_offset) {
        Some((src_port, dst_port)) => port_allowed(filter, src_port, dst_port),
        // Nothing to match a port list against - the IP list has already decided.
        None => true,
    }
}
