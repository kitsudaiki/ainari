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

use aya_ebpf::{bindings::xdp_action, macros::xdp, programs::XdpContext};
use network_types::eth::{EthHdr, EtherType};

use torii_common::{ROUTE_ACTION_ENCAP, ROUTE_ACTION_KERNEL};

use arp::handle_arp_request;
use decap::{is_tunnel_packet, process_tunnel_packet};
use encap::encap_and_redirect;
use filter::filter_allows;
use forward::redirect_local;
use maps::{is_floating_ip, is_uplink, lookup_route, uplink_mode};
use nat::{apply_dnat, apply_snat, destination_ip};
use utils::ptr_at;

/// Processes incoming packets on overlay network interfaces.
///
/// This XDP program evaluates traffic entering through virtual overlay interfaces (like TAP
/// devices). It translates destination IP addresses via DNAT if required, checks the central
/// `ROUTE_MAP`, and either redirects the packet locally or encapsulates it in a UDP tunnel
/// to transit the underlay network.
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
/// Every matched route is guarded by its packet filter. A route whose
/// include-lists are empty carries everything, which is the state a freshly
/// created route is in; as soon as the control plane adds an IP range or a port
/// to a route, packets that are named by none of its entries are dropped here.
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

    let eth_type = unsafe { core::ptr::read_unaligned(ethhdr).ether_type };

    let uplink_mode = uplink_mode();
    let from_uplink = uplink_mode && is_uplink(ctx.ingress_ifindex() as u32);

    // Answer ARP requests locally on every interface served by the responder.
    if let Some(action) = handle_arp_request(&ctx, eth_type) {
        return action;
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

    // Apply DNAT if necessary. Returns the true Target IP (or original IP if no DNAT).
    // The single gateway setup translates only what enters through the uplink,
    // so a VM reaches the floating IP of another VM through the outside, just
    // like in the split setup, and both of them see consistent addresses.
    let dest_ip = if !uplink_mode || from_uplink {
        apply_dnat(&ctx, eth_type)
    } else {
        destination_ip(&ctx, eth_type)
    };

    if let Some(dest_ip) = dest_ip
        && let Some((route_key, target)) = lookup_route(dest_ip)
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
                apply_snat(&ctx, eth_type);
            }
            return redirect_local(&ctx, &target);
        }
    }

    xdp_action::XDP_PASS
}

/// Processes incoming packets on the physical underlay interface.
///
/// This XDP program attaches to the primary host interface (e.g., `eth0`). Its
/// role is to identify encapsulated traffic (UDP port 5555) destined for our virtual
/// network, unwrap it, and route the internal payload to the appropriate TAP interface.
/// Normal traffic is passed through unaffected.
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
