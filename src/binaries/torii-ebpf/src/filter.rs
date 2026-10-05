use network_types::eth::{EthHdr, EtherType};
use network_types::ip::IpProto;
use torii_common::{FILTER_MAX_IP_RANGES, FILTER_MAX_PORT_RANGES, RouteFilter, RouteKey};

use crate::headers::{Ipv4Hdr, TcpHdr, UdpHdr};
use crate::maps::{lookup_egress_filter, lookup_filter};
use crate::utils::{PacketContext, ptr_at};

/// Checks whether an address is named by the IP include-list.
///
/// An empty list means "no restriction", which is the state every filter starts
/// in. As soon as one range is added the list becomes exclusive: only addresses
/// inside one of its ranges are carried.
///
/// # Arguments
/// * `filter` - The filter the packet is checked against
/// * `ip` - Source address (ingress) or destination address (egress) of the
///   packet in host byte order
///
/// # Returns
/// `true` when the list is empty or one of its ranges contains the address
#[inline(always)]
fn ip_allowed(filter: &RouteFilter, ip: u32) -> bool {
    if filter.ip_range_count == 0 {
        return true;
    }

    for i in 0..FILTER_MAX_IP_RANGES {
        if i as u32 >= filter.ip_range_count {
            break;
        }
        let range = filter.ip_ranges[i];
        if ip >= range.start && ip <= range.end {
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
/// * `filter` - The filter the packet is checked against
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
/// * `ctx` - The XDP or TC context of the packet
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
    ctx: &impl PacketContext,
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

/// Applies the ingress packet filter of a route to a packet that matched it.
///
/// The check runs directly before the forwarding decision is carried out, so it
/// sees the packet the way it will leave the gateway - floating IP translation
/// has already happened at that point. The IP include-list is matched against
/// the source address of the packet. Routes without a filter, which is every
/// route until the control plane attaches one, carry everything.
///
/// # Arguments
/// * `ctx` - The XDP or TC context of the packet
/// * `eth_type` - The already parsed EtherType of the frame
/// * `route_key` - The `(vni, destination)` key the route was matched under
///
/// # Returns
/// `true` when the packet may be forwarded, `false` when it has to be dropped
#[inline(always)]
pub fn filter_allows(ctx: &impl PacketContext, eth_type: EtherType, route_key: RouteKey) -> bool {
    match lookup_filter(route_key) {
        Some(filter) => packet_allowed(ctx, eth_type, filter, false),
        None => true,
    }
}

/// Applies the egress packet filter of a VM to a packet the VM has sent.
///
/// The check runs as soon as the packet arrived on the TAP device of the VM,
/// before anything is translated, so it sees the packet the way the VM wrote
/// it. The IP include-list is matched against the destination address of the
/// packet. A TAP device without a filter carries everything.
///
/// # Arguments
/// * `ctx` - The XDP or TC context of the packet
/// * `eth_type` - The already parsed EtherType of the frame
/// * `ifindex` - The interface the packet arrived on
///
/// # Returns
/// `true` when the packet may be forwarded, `false` when it has to be dropped
#[inline(always)]
pub fn egress_filter_allows(ctx: &impl PacketContext, eth_type: EtherType, ifindex: u32) -> bool {
    match lookup_egress_filter(ifindex) {
        Some(filter) => packet_allowed(ctx, eth_type, filter, true),
        None => true,
    }
}

/// Checks a packet against a packet filter.
///
/// Two things pass unconditionally:
///
/// * everything that is not IPv4, i.e. the ARP traffic the datapath needs to
///   keep the unnumbered links alive
/// * protocols without ports (ICMP and friends) with respect to the *port*
///   list; they are still matched against the IP list
///
/// A packet whose IPv4 header cannot be parsed is rejected: a filter that
/// cannot be evaluated must not turn into a hole.
///
/// # Arguments
/// * `ctx` - The XDP or TC context of the packet
/// * `eth_type` - The already parsed EtherType of the frame
/// * `filter` - The filter the packet is checked against
/// * `match_destination` - `true` to match the IP list against the destination
///   address of the packet, `false` to match it against its source address
///
/// # Returns
/// `true` when the packet may be forwarded, `false` when it has to be dropped
#[inline(always)]
fn packet_allowed(
    ctx: &impl PacketContext,
    eth_type: EtherType,
    filter: &RouteFilter,
    match_destination: bool,
) -> bool {
    if eth_type != EtherType::Ipv4 {
        return true;
    }

    let ipv4 = match ptr_at::<Ipv4Hdr>(ctx, EthHdr::LEN) {
        Ok(ptr) => unsafe { core::ptr::read_unaligned(ptr) },
        Err(_) => return false,
    };

    let ip = if match_destination {
        ipv4.dst_addr
    } else {
        ipv4.src_addr
    };
    if !ip_allowed(filter, u32::from_be(ip)) {
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
