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
