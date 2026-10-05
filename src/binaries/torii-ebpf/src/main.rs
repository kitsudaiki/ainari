#![no_std]
#![no_main]

mod arp;
mod decap; // Add the new module
mod encap;
mod filter;
mod forward;
mod headers;
mod maps;
mod nat;
mod utils;

use aya_ebpf::{
    bindings::{TC_ACT_OK, TC_ACT_SHOT, xdp_action},
    macros::{classifier, xdp},
    programs::{TcContext, XdpContext},
};
use network_types::eth::{EthHdr, EtherType};

use torii_common::{ROUTE_ACTION_ENCAP, ROUTE_ACTION_KERNEL, ROUTE_ACTION_LOCAL};

use arp::handle_arp_request;
use decap::{is_tunnel_packet, process_tunnel_packet};
use encap::encap_and_redirect;
use filter::{egress_filter_allows, filter_allows};
use forward::redirect_local;
use maps::{is_floating_ip, is_uplink, lookup_iface, lookup_route, uplink_mode};
use nat::{apply_dnat, apply_snat, destination_ip};
use utils::ptr_at;

/// Processes incoming packets on overlay network interfaces.
///
/// This XDP program evaluates traffic entering through virtual overlay interfaces (like TAP
/// devices). It translates destination IP addresses via DNAT if required, checks the central
/// `ROUTE_MAP`, and either redirects the packet locally or encapsulates it in a VXLAN tunnel
/// to transit the underlay network.
///
/// The tenant of the packet is taken from the interface it arrived on, never
/// from anything inside the packet. That is what lets two VMs on this host carry
/// the very same address: they sit on different ports, so their packets are
/// looked up in different halves of the routing map. The one exception is a port
/// marked as a floating IP port, where the globally unique floating IP names the
/// tenant instead - which is how traffic from the outside world finds its way
/// into a tenant in the first place.
///
/// ARP requests coming from a VM are terminated right here: the TAP devices carry
/// no IP address, so the program answers them itself instead of relying on the
/// host stack. This keeps the host side of every VM link completely unnumbered
/// and therefore free of address conflicts between VMs of the same subnet.
///
/// Destinations that are reached over an IPsec protected connection carry
/// `ROUTE_ACTION_KERNEL` and are passed up instead of being encapsulated here:
/// the ESP transformation lives in the kernel, so those packets have to take
/// the regular forwarding path. Everything else stays in the eBPF datapath.
///
/// Every matched route is guarded by its ingress packet filter. A route whose
/// include-lists are empty carries everything, which is the state a freshly
/// created route is in; as soon as the control plane adds an IP range or a port
/// to a route, packets that are named by none of its entries are dropped here.
/// What a VM sends is checked against the egress filter of its TAP device
/// before anything else happens with it.
///
/// In the single gateway setup (uplink mode) the same program also serves the
/// uplink towards the outside. There the floating IP NAT is done in both
/// directions by this program alone: DNAT for what enters through the uplink,
/// SNAT for what leaves through it. Without uplink mode nothing of that applies
/// and the program behaves exactly like in the split setup.
///
/// # Arguments
/// * `ctx` - The eBPF XDP Context containing raw packet data and metadata
///
/// # Returns
/// An eBPF `xdp_action` determining whether to redirect, pass, or drop the packet
#[xdp]
pub fn overlay_ingress(ctx: XdpContext) -> u32 {
    let ethhdr = match ptr_at::<EthHdr>(&ctx, 0) {
        Ok(hdr) => hdr,
        Err(_) => return xdp_action::XDP_PASS,
    };

    let eth_type = match unsafe { core::ptr::read_unaligned(ethhdr).ether_type() } {
        Ok(eth_type) => eth_type,
        Err(_) => return xdp_action::XDP_PASS,
    };

    let uplink_mode = uplink_mode();
    let from_uplink = uplink_mode && is_uplink(ctx.ingress_ifindex() as u32);

    // The tenant of this packet is a property of the port it came in on.
    let iface = lookup_iface(ctx.ingress_ifindex() as u32);

    // Answer ARP requests locally on every interface served by the responder.
    if let Some(action) = handle_arp_request(&ctx, eth_type, iface.vni) {
        return action;
    }

    // A packet sent by a VM has to pass the egress filter of the VM first. The
    // filter is keyed by the TAP device the packet came in on, so every other
    // interface, including the uplink, has none.
    if !egress_filter_allows(&ctx, eth_type, ctx.ingress_ifindex() as u32) {
        return xdp_action::XDP_DROP;
    }

    // Single gateway setup: from the uplink only the floating IPs lead into the
    // virtual network. Everything else arriving there - the ARP traffic of that
    // segment, packets addressed to the gateway host itself - is the kernel's.
    if from_uplink {
        if eth_type != EtherType::Ipv4 {
            return xdp_action::XDP_PASS;
        }
        match destination_ip(&ctx, eth_type) {
            Some(ip) if is_floating_ip(ip) => {}
            _ => return xdp_action::XDP_PASS,
        }
    }

    // Apply DNAT if necessary. Returns the tenant and the true Target IP (or the
    // port's tenant and the original IP if no DNAT applies).
    //
    // The translation runs on the ports that face the outside world and nowhere
    // else: in the single gateway setup that is the uplink, in the split setup
    // every interface the control plane did not claim for a tenant. A TAP is
    // never one of them, so a VM reaches the floating IP of another VM through
    // the outside instead of stepping into its tenant here.
    let translate = (!uplink_mode || from_uplink) && iface.does_fip();

    if let Some((vni, dest_ip)) = apply_dnat(&ctx, eth_type, iface.vni, translate)
        && let Some((route_key, target)) = lookup_route(vni, dest_ip)
    {
        // The filter belongs to the route, so it guards every way out of it -
        // the overlay, the kernel path of an encrypted destination and the
        // local delivery alike.
        if !filter_allows(&ctx, eth_type, route_key) {
            return xdp_action::XDP_DROP;
        }

        if target.action == ROUTE_ACTION_ENCAP {
            return encap_and_redirect(&ctx, &target);
        } else if target.action == ROUTE_ACTION_KERNEL {
            // IPsec protected destination: let the kernel encrypt and route it.
            return xdp_action::XDP_PASS;
        } else {
            // Single gateway setup: leaving through the uplink means leaving the
            // virtual network, so the VM is masked behind its floating IP. This
            // happens after the filter, which therefore still sees the VM.
            if uplink_mode && is_uplink(target.ifindex) {
                apply_snat(&ctx, eth_type, vni);
            }
            return redirect_local(&ctx, &target);
        }
    }

    xdp_action::XDP_PASS
}

/// Processes incoming packets on the physical underlay interface.
///
/// This XDP program attaches to the primary host interface (e.g., `eth0`). Its
/// role is to identify encapsulated VXLAN traffic destined for our virtual network,
/// unwrap it, and route the internal payload to the appropriate TAP interface.
/// Normal traffic is passed through unaffected.
///
/// The tenant of an arriving packet is read out of its VXLAN header before the
/// outer headers are stripped. Without it the inner address would be ambiguous
/// whenever two local VMs share it.
///
/// # Arguments
/// * `ctx` - The eBPF XDP Context containing raw packet data and metadata
///
/// # Returns
/// An eBPF `xdp_action` code specifying the next step for the packet
#[xdp]
pub fn underlay_ingress(ctx: XdpContext) -> u32 {
    // If this packet is encapsulated VM traffic, process and route it locally
    if is_tunnel_packet(&ctx) {
        return process_tunnel_packet(&ctx);
    }

    // Otherwise, let standard host networking handle it
    xdp_action::XDP_PASS
}

/// Number of bytes at the start of a packet, which `tap_egress` needs in the linear part of the
/// socket buffer: the Ethernet header, an IPv4 header with the maximum of options and the ports
/// of the transport header.
const TAP_EGRESS_PULL_LEN: u32 = 14 + 60 + 4;

/// Applies the ingress packet filter to the packets the kernel sends into a TAP device.
///
/// This TC program attaches to the egress of every TAP device. Most packets towards a VM are
/// redirected into its TAP device by the XDP programs, which already applied the filter and
/// don't pass the TC layer at all. Decrypted IPsec traffic is different: the kernel decrypts it
/// and routes it into the TAP device itself, so no XDP program ever sees it. Without this program
/// the ingress filter of a VM would be bypassed by everything, which arrives encrypted.
///
/// The packet is matched against the route of the tenant of the TAP device, the same way the XDP
/// programs do it. Only a route, which leads into this very TAP device, is considered, so the
/// filter of another VM is never applied here.
///
/// # Arguments
/// * `ctx` - The eBPF TC Context containing the socket buffer of the packet
///
/// # Returns
/// `TC_ACT_SHOT` if the filter rejects the packet, otherwise `TC_ACT_OK`
#[classifier]
pub fn tap_egress(ctx: TcContext) -> i32 {
    // On egress the interface of the socket buffer is the device the packet is sent on.
    let ifindex = unsafe { (*ctx.skb.skb).ifindex };

    // The headers may lie outside of the linear part of the buffer, where the program can't
    // read them. A packet shorter than the requested length is pulled completely.
    let pull_len = core::cmp::min(ctx.len(), TAP_EGRESS_PULL_LEN);
    if ctx.pull_data(pull_len).is_err() {
        return TC_ACT_SHOT as i32;
    }

    let ethhdr = match ptr_at::<EthHdr>(&ctx, 0) {
        Ok(hdr) => hdr,
        Err(_) => return TC_ACT_OK as i32,
    };
    let eth_type = match unsafe { core::ptr::read_unaligned(ethhdr).ether_type() } {
        Ok(eth_type) => eth_type,
        Err(_) => return TC_ACT_OK as i32,
    };

    let iface = lookup_iface(ifindex);
    if let Some(dest_ip) = destination_ip(&ctx, eth_type)
        && let Some((route_key, target)) = lookup_route(iface.vni, dest_ip)
        && target.action == ROUTE_ACTION_LOCAL
        && target.ifindex == ifindex
        && !filter_allows(&ctx, eth_type, route_key)
    {
        return TC_ACT_SHOT as i32;
    }

    TC_ACT_OK as i32
}

/// Fallback handler invoked when the eBPF program encounters an unrecoverable error.
///
/// eBPF environments do not support standard panic unwinding. This function
/// guarantees that the program safely aborts without crashing the kernel execution context.
///
/// # Arguments
/// * `_info` - Information about the panic context
///
/// # Returns
/// This function diverges and never returns
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
