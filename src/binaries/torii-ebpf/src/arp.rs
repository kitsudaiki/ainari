use aya_ebpf::bindings::xdp_action;
use aya_ebpf::programs::XdpContext;
use network_types::eth::{EthHdr, EtherType};
use torii_common::ROUTE_ACTION_LOCAL;

use crate::headers::ArpHdr;
use crate::maps::{is_floating_ip, lookup_arp_proxy, lookup_route_exact, lookup_uplink};
use crate::utils::ptr_at_mut;

/// Hardware type for Ethernet inside an ARP header.
const ARP_HTYPE_ETHER: u16 = 1;
/// Protocol type for IPv4 inside an ARP header.
const ARP_PTYPE_IPV4: u16 = 0x0800;
/// ARP operation code of a request.
const ARP_OP_REQUEST: u16 = 1;
/// ARP operation code of a reply.
const ARP_OP_REPLY: u16 = 2;

/// Answers ARP requests on behalf of the whole virtual network (proxy ARP).
///
/// TAP devices carry no IP address and no subnet at all, therefore the host
/// kernel can never reply to the ARP requests a VM sends for its default
/// gateway or for a neighbour of its own subnet. This function performs that
/// job entirely in XDP: every ARP request received on a registered TAP is
/// turned into a reply carrying the TAP's own MAC address and bounced straight
/// back to the VM with `XDP_TX`.
///
/// Because the answer is always the same MAC, the VM sends *all* of its traffic
/// to the gateway, where the eBPF `ROUTE_MAP` decides what happens with it. That
/// removes every IP address from the host side of the link and makes it
/// possible to run several VMs of the *same* subnet on one host.
///
/// The route probe below is made inside the tenant of the ingress interface, so
/// two VMs that carry the very same address in different tenants each get the
/// answer that belongs to their own side of the map.
///
/// Requests that must not be answered are ignored so the VM can still boot and
/// defend its own address:
/// * ARP probes (`spa == 0`) and gratuitous announcements (`spa == tpa`), which
///   are used for duplicate address detection
/// * requests for an address that is routed back onto the very same interface,
///   i.e. the address of the VM asking
///
/// In the single gateway setup the uplink is served as well, but only for the
/// floating IPs: the gateway answers them with the MAC of the uplink, so the
/// outside reaches a VM without the VM itself having to take part in ARP.
/// Every other address on the uplink is left to the kernel.
///
/// # Arguments
/// * `ctx` - The XDP context of the received packet
/// * `eth_type` - The already parsed EtherType of the frame
/// * `vni` - The tenant of the interface the request arrived on
///
/// # Returns
/// `Some(XDP_TX)` when a reply was generated in place, otherwise `None` so the
/// caller continues with the regular routing pipeline.
#[inline(always)]
pub fn handle_arp_request(ctx: &XdpContext, eth_type: EtherType, vni: u32) -> Option<u32> {
    if eth_type != EtherType::Arp {
        return None;
    }

    // Only interfaces explicitly registered by the control plane are served:
    // the TAP devices, and in the single gateway setup the uplinks. The
    // underlay and the veth uplink of the split setup keep their ordinary
    // forwarding behaviour.
    let ingress = ctx.ingress_ifindex() as u32;
    let (proxy, on_uplink) = match lookup_arp_proxy(ingress) {
        Some(proxy) => (proxy, false),
        None => (lookup_uplink(ingress)?, true),
    };

    let arp_ptr = ptr_at_mut::<ArpHdr>(ctx, EthHdr::LEN).ok()?;
    let arp = unsafe { core::ptr::read_unaligned(arp_ptr) };

    // Restrict ourselves to plain IPv4-over-Ethernet requests.
    if arp.htype != u16::to_be(ARP_HTYPE_ETHER) || arp.ptype != u16::to_be(ARP_PTYPE_IPV4) {
        return None;
    }
    if arp.hlen != 6 || arp.plen != 4 || arp.oper != u16::to_be(ARP_OP_REQUEST) {
        return None;
    }

    let spa = u32::from_be(arp.spa);
    let tpa = u32::from_be(arp.tpa);

    // Never interfere with duplicate address detection of the VM itself.
    if spa == 0 || spa == tpa {
        return None;
    }

    if on_uplink {
        // On the uplink the gateway speaks for its floating IPs only.
        if !is_floating_ip(tpa) {
            return None;
        }
    } else {
        // Never claim an address that lives on the asking side of the link.
        if proxy.vm_ip != 0 && tpa == proxy.vm_ip {
            return None;
        }
        if let Some(route) = lookup_route_exact(vni, tpa)
            && route.action == ROUTE_ACTION_LOCAL
            && route.ifindex == ingress
        {
            return None;
        }
    }

    // Turn the request into a reply: the sender becomes the target and the
    // TAP device (or the uplink) itself becomes the sender of the answer.
    let eth_ptr = ptr_at_mut::<EthHdr>(ctx, 0).ok()?;
    let mut eth = unsafe { core::ptr::read_unaligned(eth_ptr) };
    eth.dst_addr = eth.src_addr;
    eth.src_addr = proxy.mac;
    eth.ether_type = EtherType::Arp;
    unsafe { core::ptr::write_unaligned(eth_ptr, eth) };

    let mut reply = arp;
    reply.oper = u16::to_be(ARP_OP_REPLY);
    reply.tha = arp.sha;
    reply.sha = proxy.mac;
    reply.tpa = arp.spa;
    reply.spa = arp.tpa;
    unsafe { core::ptr::write_unaligned(arp_ptr, reply) };

    // Bounce the freshly built reply back out of the interface it came from.
    Some(xdp_action::XDP_TX)
}
