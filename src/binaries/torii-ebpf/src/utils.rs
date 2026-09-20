use crate::headers::Ipv4Hdr;
use aya_ebpf::programs::XdpContext;
use core::mem;

/// Retrieves a validated, read-only pointer to a struct within the packet data buffer.
///
/// This helper performs critical boundary checks against the `data_end` pointer provided
/// by the kernel. Without these checks, the eBPF verifier will reject the program.
///
/// # Arguments
/// * `ctx` - The current XDP context containing packet data
/// * `offset` - The byte offset from the start of the packet where the struct begins
///
/// # Returns
/// A `Result` containing a safe `*const T` pointer, or an `Err(())` if out-of-bounds
#[inline(always)]
pub fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();
    if start + offset + len > end {
        return Err(());
    }
    Ok((start + offset) as *const T)
}

/// Retrieves a validated, mutable pointer to a struct within the packet data buffer.
///
/// Functionally identical to `ptr_at`, but yields a mutable pointer allowing inline
/// payload modifications for processes like NAT rewriting or Tunnel Encapsulation.
///
/// # Arguments
/// * `ctx` - The current XDP context containing packet data
/// * `offset` - The byte offset from the start of the packet where the struct begins
///
/// # Returns
/// A `Result` containing a safe `*mut T` pointer, or an `Err(())` if out-of-bounds
#[inline(always)]
pub fn ptr_at_mut<T>(ctx: &XdpContext, offset: usize) -> Result<*mut T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = core::mem::size_of::<T>();
    if start + offset + len > end {
        return Err(());
    }
    Ok((start + offset) as *mut T)
}

/// Calculates the standard IPv4 header checksum.
///
/// Sums the 16-bit words of the IPv4 header and folds the result according to RFC 1071.
/// Used when modifying IP properties like TTL or Address.
///
/// # Arguments
/// * `hdr` - A reference to the populated `Ipv4Hdr` structure
///
/// # Returns
/// A `u16` representing the calculated checksum in big-endian format
#[inline(always)]
pub fn ipv4_checksum(hdr: &Ipv4Hdr) -> u16 {
    let mut sum: u32 = 0;
    let b = unsafe { core::slice::from_raw_parts(hdr as *const _ as *const u8, 20) };

    sum += u16::from_be_bytes([b[0], b[1]]) as u32;
    sum += u16::from_be_bytes([b[2], b[3]]) as u32;
    sum += u16::from_be_bytes([b[4], b[5]]) as u32;
    sum += u16::from_be_bytes([b[6], b[7]]) as u32;
    sum += u16::from_be_bytes([b[8], b[9]]) as u32;
    sum += u16::from_be_bytes([b[10], b[11]]) as u32;
    sum += u16::from_be_bytes([b[12], b[13]]) as u32;
    sum += u16::from_be_bytes([b[14], b[15]]) as u32;
    sum += u16::from_be_bytes([b[16], b[17]]) as u32;
    sum += u16::from_be_bytes([b[18], b[19]]) as u32;

    sum = (sum & 0xffff) + (sum >> 16);
    sum = (sum & 0xffff) + (sum >> 16);

    u16::to_be(!(sum as u16))
}

/// Incrementally updates a Layer 4 checksum based on a 32-bit modification.
///
/// RFC 1624 implementation to adjust TCP/UDP checksums without recalculating the
/// entire payload. This is heavily utilized during NAT translation where only
/// the Source or Destination IPs change.
///
/// # Arguments
/// * `csum` - A mutable reference to the existing 16-bit checksum
/// * `old` - The 32-bit value being removed from the header (e.g. old IP)
/// * `new` - The 32-bit value being inserted into the header (e.g. new IP)
///
/// # Returns
/// None. The checksum is mutated in place.
#[inline(always)]
pub fn csum_replace4(csum: &mut u16, old: u32, new: u32) {
    let mut sum: u32 = u32::from(!*csum);

    let old_1 = (old >> 16) as u16;
    let old_2 = (old & 0xffff) as u16;
    let new_1 = (new >> 16) as u16;
    let new_2 = (new & 0xffff) as u16;

    sum = sum
        .wrapping_add(!old_1 as u32)
        .wrapping_add(!old_2 as u32)
        .wrapping_add(new_1 as u32)
        .wrapping_add(new_2 as u32);

    sum = (sum & 0xffff) + (sum >> 16);
    sum = (sum & 0xffff) + (sum >> 16);
    *csum = !(sum as u16);
}
