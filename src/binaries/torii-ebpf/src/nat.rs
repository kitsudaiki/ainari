use crate::headers::{ArpHdr, Ipv4Hdr, TcpHdr, UdpHdr};
use crate::maps::{FIP_DNAT_MAP, FIP_SNAT_MAP};
use crate::utils::{csum_replace4, ipv4_checksum, ptr_at, ptr_at_mut};
use aya_ebpf::programs::XdpContext;
use network_types::eth::{EthHdr, EtherType};
use network_types::icmp::IcmpHdr;
use network_types::ip::IpProto;

/// Applies Destination Network Address Translation (DNAT) to incoming traffic.
///
/// Mutates the packet inline by altering the destination IP address if it matches a registered
/// Floating IP. Automatically fixes the Layer 3 (IPv4) and Layer 4 (TCP/UDP) checksums
/// to ensure the packet remains valid through the rest of the network stack. Handles both
/// IPv4 payloads and ARP resolution requests.
///
/// # Arguments
/// * `ctx` - The XDP packet context (must be mutable to rewrite headers)
/// * `eth_type` - The parsed protocol type of the inner payload
///
/// # Returns
/// An `Option<u32>` containing the translated internal destination IP, or the original if unmapped
#[inline(always)]
pub fn apply_dnat(ctx: &XdpContext, eth_type: EtherType) -> Option<u32> {
    // -----------------------------------------------------------------------
    // IPv4 PACKET PROCESSING
    // -----------------------------------------------------------------------
    if eth_type == EtherType::Ipv4 {
        // Attempt to get a mutable pointer to the IPv4 header.
        // It resides immediately after the Ethernet header (offset: EthHdr::LEN).
        if let Ok(ipv4_ptr) = ptr_at_mut::<Ipv4Hdr>(ctx, EthHdr::LEN) {
            // Due to eBPF packet buffer constraints, we must read unaligned memory safely.
            let mut ipv4 = unsafe { core::ptr::read_unaligned(ipv4_ptr) };

            // Extract the original destination IP (in big-endian/network byte order).
            let mut dst_val = u32::from_be(ipv4.dst_addr);

            // Check if this destination IP is a known Floating IP (FIP) requiring translation.
            if let Some(&internal_ip) = unsafe { FIP_DNAT_MAP.get(dst_val) } {
                // 1. Rewrite the Layer 3 Destination Address
                ipv4.dst_addr = u32::to_be(internal_ip);

                // 2. Recalculate the IPv4 Header Checksum
                // The L3 checksum must be zeroed out before recalculating.
                ipv4.check = 0;
                ipv4.check = ipv4_checksum(&ipv4);

                // Write the modified IPv4 header back into the packet buffer.
                unsafe { core::ptr::write_unaligned(ipv4_ptr, ipv4) };

                // 3. Layer 4 Checksum Updates
                // Determine where the L4 header starts. The Internet Header Length (IHL)
                // is stored in the lower 4 bits of `version_ihl` and represents 32-bit words.
                let l4_offset = EthHdr::LEN + ((ipv4.version_ihl & 0x0F) * 4) as usize;

                if ipv4.protocol == IpProto::Tcp as u8 {
                    // Update TCP Checksum
                    if let Ok(tcp) = ptr_at_mut::<TcpHdr>(ctx, l4_offset) {
                        let mut check = unsafe { (*tcp).check };
                        // We use an incremental checksum update (RFC 1624) instead of
                        // recalculating the entire payload, which is too slow for XDP.
                        csum_replace4(&mut check, u32::to_be(dst_val), u32::to_be(internal_ip));
                        unsafe { (*tcp).check = check };
                    }
                } else if ipv4.protocol == IpProto::Udp as u8 {
                    // Update UDP Checksum
                    if let Ok(udp) = ptr_at_mut::<UdpHdr>(ctx, l4_offset) {
                        let mut check = unsafe { (*udp).check };
                        // A UDP checksum of 0 means "no checksum used" in IPv4.
                        // Only update it if it's actually actively being used.
                        if check != 0 {
                            csum_replace4(&mut check, u32::to_be(dst_val), u32::to_be(internal_ip));
                            unsafe { (*udp).check = check };
                        }
                    }
                } else if ipv4.protocol == IpProto::Icmp as u8 {
                    // Handle ICMP Checksum
                    if let Ok(_icmp) = ptr_at_mut::<IcmpHdr>(ctx, l4_offset) {
                        // Note: ICMPv4 does NOT include the IP pseudo-header in its checksum.
                        // Because we only changed the outer IP address (and not the ICMP payload),
                        // the L4 checksum remains completely valid. No csum_replace4 is needed.
                    }
                }

                // Update our tracking variable to the new internal IP for the caller.
                dst_val = internal_ip;
            }
            return Some(dst_val);
        }
    }
    // -----------------------------------------------------------------------
    // ARP PACKET PROCESSING
    // -----------------------------------------------------------------------
    else if eth_type == EtherType::Arp
        && let Ok(arp_ptr) = ptr_at_mut::<ArpHdr>(ctx, EthHdr::LEN)
    {
        let mut arp = unsafe { core::ptr::read_unaligned(arp_ptr) };

        // Extract the Target Protocol Address (TPA) - the IP being asked about.
        let mut tpa_val = u32::from_be(arp.tpa);

        // If the ARP request is looking for a Floating IP, rewrite it so the
        // internal VM actually recognizes it and responds.
        if let Some(&internal_ip) = unsafe { FIP_DNAT_MAP.get(tpa_val) } {
            arp.tpa = u32::to_be(internal_ip);
            unsafe { core::ptr::write_unaligned(arp_ptr, arp) };
            tpa_val = internal_ip;
        }
        return Some(tpa_val);
    }
    None
}

/// Applies Source Network Address Translation (SNAT) to outgoing traffic.
///
/// Modifies the packet inline, masking the internal source IP address with its assigned
/// public Floating IP. Similar to `apply_dnat`, it comprehensively rewrites Layer 3 and
/// Layer 4 checksums to prevent packet drop by intermediate firewalls or the receiver.
///
/// # Arguments
/// * `ctx` - The XDP packet context
/// * `eth_type` - The parsed protocol type of the packet
///
/// # Returns
/// An `Option<u32>` containing the true packet destination IP (unaltered), or None if invalid
#[inline(always)]
pub fn apply_snat(ctx: &XdpContext, eth_type: EtherType) -> Option<u32> {
    // We will track the destination IP so the routing logic knows where to send this later.
    let mut dest_ip = None;

    // -----------------------------------------------------------------------
    // IPv4 PACKET PROCESSING
    // -----------------------------------------------------------------------
    if eth_type == EtherType::Ipv4 {
        if let Ok(inner_ip_ptr) = ptr_at_mut::<Ipv4Hdr>(ctx, EthHdr::LEN) {
            let mut inner_ip = unsafe { core::ptr::read_unaligned(inner_ip_ptr) };

            // Store the true destination IP before we do any translation.
            dest_ip = Some(u32::from_be(inner_ip.dst_addr));

            // Extract the original source IP (the private VM IP).
            let src_val = u32::from_be(inner_ip.src_addr);

            // Check if this source IP should be masked behind a Floating IP (FIP).
            if let Some(&fip) = unsafe { FIP_SNAT_MAP.get(src_val) } {
                // 1. Rewrite the Layer 3 Source Address (Masking)
                inner_ip.src_addr = u32::to_be(fip);

                // 2. Recalculate the IPv4 Header Checksum
                inner_ip.check = 0;
                inner_ip.check = ipv4_checksum(&inner_ip);
                unsafe { core::ptr::write_unaligned(inner_ip_ptr, inner_ip) };

                // Calculate the start of the Layer 4 header (TCP/UDP/ICMP).
                let l4_offset = EthHdr::LEN + ((inner_ip.version_ihl & 0x0F) * 4) as usize;

                // 3. Layer 4 Checksum Updates
                if inner_ip.protocol == IpProto::Tcp as u8 {
                    // Update TCP Checksum using incremental replacement for the new Source IP.
                    if let Ok(tcp) = ptr_at_mut::<TcpHdr>(ctx, l4_offset) {
                        let mut check = unsafe { (*tcp).check };
                        csum_replace4(&mut check, u32::to_be(src_val), u32::to_be(fip));
                        unsafe { (*tcp).check = check };
                    }
                } else if inner_ip.protocol == IpProto::Udp as u8 {
                    // Update UDP Checksum using incremental replacement.
                    if let Ok(udp) = ptr_at_mut::<UdpHdr>(ctx, l4_offset) {
                        let mut check = unsafe { (*udp).check };
                        // Skip if UDP checksumming is disabled (0).
                        if check != 0 {
                            csum_replace4(&mut check, u32::to_be(src_val), u32::to_be(fip));
                            unsafe { (*udp).check = check };
                        }
                    }
                } else if inner_ip.protocol == IpProto::Icmp as u8 {
                    // Handle ICMP Checksum
                    if let Ok(_icmp) = ptr_at_mut::<IcmpHdr>(ctx, l4_offset) {
                        // Pass-through without csum rewrite for the same reasons as DNAT:
                        // ICMPv4 checksums exclude the IP header.
                    }
                }
            }
        }
    }
    // -----------------------------------------------------------------------
    // ARP PACKET PROCESSING
    // -----------------------------------------------------------------------
    else if eth_type == EtherType::Arp
        && let Ok(arp_ptr) = ptr_at_mut::<ArpHdr>(ctx, EthHdr::LEN)
    {
        let mut arp = unsafe { core::ptr::read_unaligned(arp_ptr) };

        // Extract the Sender Protocol Address (SPA) - who is claiming this ARP.
        let spa_val = u32::from_be(arp.spa);

        // If an internal VM is sending an ARP reply/request, mask its internal IP
        // with the public Floating IP so the external network accepts it.
        if let Some(&fip) = unsafe { FIP_SNAT_MAP.get(spa_val) } {
            arp.spa = u32::to_be(fip);
            unsafe { core::ptr::write_unaligned(arp_ptr, arp) };
        }

        // Store the Target Protocol Address (TPA) for routing purposes.
        dest_ip = Some(u32::from_be(arp.tpa));
    }

    // Return the destination IP (whether we modified the packet or not) so
    // the routing pipeline can forward the packet correctly.
    dest_ip
}

/// Reads the destination address of a packet without translating anything.
///
/// The counterpart of [`apply_dnat`] and [`apply_snat`] for the paths of the
/// single gateway setup that must leave the addresses alone: IPv4 packets yield
/// their destination, ARP packets the address they ask for.
///
/// # Arguments
/// * `ctx` - The XDP packet context
/// * `eth_type` - The parsed protocol type of the packet
///
/// # Returns
/// An `Option<u32>` containing the destination IP, or None if there is none
#[inline(always)]
pub fn destination_ip(ctx: &XdpContext, eth_type: EtherType) -> Option<u32> {
    if eth_type == EtherType::Ipv4 {
        let ipv4 = ptr_at::<Ipv4Hdr>(ctx, EthHdr::LEN).ok()?;
        return Some(u32::from_be(
            unsafe { core::ptr::read_unaligned(ipv4) }.dst_addr,
        ));
    }
    if eth_type == EtherType::Arp {
        let arp = ptr_at::<ArpHdr>(ctx, EthHdr::LEN).ok()?;
        return Some(u32::from_be(unsafe { core::ptr::read_unaligned(arp) }.tpa));
    }
    None
}
