/// Standard IPv4 packet header mapping.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct Ipv4Hdr {
    pub version_ihl: u8,
    pub tos: u8,
    pub tot_len: u16,
    pub id: u16,
    pub frag_off: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub check: u16,
    pub src_addr: u32,
    pub dst_addr: u32,
}

impl Ipv4Hdr {
    pub const LEN: usize = 20;
}

/// Standard UDP packet header mapping.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct UdpHdr {
    pub source: u16,
    pub dest: u16,
    pub len: u16,
    pub check: u16,
}

impl UdpHdr {
    pub const LEN: usize = 8;
}

/// Standard TCP packet header mapping.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct TcpHdr {
    pub source: u16,
    pub dest: u16,
    pub seq: u32,
    pub ack_seq: u32,
    pub res1_doff_flags: u16,
    pub window: u16,
    pub check: u16,
    pub urg_ptr: u16,
}

/// Standard ARP packet header mapping.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct ArpHdr {
    pub htype: u16,
    pub ptype: u16,
    pub hlen: u8,
    pub plen: u8,
    pub oper: u16,
    pub sha: [u8; 6],
    pub spa: u32,
    pub tha: [u8; 6],
    pub tpa: u32,
}

/// VXLAN header (RFC 7348) as it sits between the outer UDP header and the
/// encapsulated Ethernet frame.
///
/// `flags` carries the `I` bit (0x08), which declares the VNI field valid -
/// every packet this gateway sends sets it. The VNI itself is 24 bits followed
/// by one reserved byte, so it is kept as raw bytes here and assembled by
/// [`Self::tenant`] instead of relying on any bit field layout.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct VxlanHdr {
    pub flags: u8,
    pub reserved: [u8; 3],
    pub vni: [u8; 3],
    pub reserved2: u8,
}

impl VxlanHdr {
    pub const LEN: usize = 8;

    /// VXLAN flag declaring the VNI field valid.
    pub const FLAG_VNI_PRESENT: u8 = 0x08;

    /// Builds a header for one tenant.
    ///
    /// # Arguments
    /// * `vni` - The tenant the encapsulated frame belongs to (24 bit)
    ///
    /// # Returns
    /// A `VxlanHdr` with the `I` bit set and the VNI in network byte order
    #[inline(always)]
    pub fn new(vni: u32) -> Self {
        Self {
            flags: Self::FLAG_VNI_PRESENT,
            reserved: [0; 3],
            vni: [(vni >> 16) as u8, (vni >> 8) as u8, vni as u8],
            reserved2: 0,
        }
    }

    /// Reads the tenant out of a received header.
    ///
    /// # Arguments
    /// None
    ///
    /// # Returns
    /// The VNI in host byte order
    #[inline(always)]
    pub fn tenant(&self) -> u32 {
        ((self.vni[0] as u32) << 16) | ((self.vni[1] as u32) << 8) | self.vni[2] as u32
    }
}
