//! IPsec key and policy handling.
//!
//! The gateway does not implement any cryptography itself: it programs the
//! kernel's xfrm stack, which carries the AES-256-GCM transformation. This
//! module owns everything that touches Security Associations and policies, so
//! the rules about when traffic is protected live in exactly one place.

use std::net::Ipv4Addr;

use crate::core::models::Connection;
use crate::core::utils::{run_ip, run_ip_output, run_ip_with_secrets};

use ainari_common::secret::Secret;

/// Writes the xfrm policies of a connection.
///
/// The policies carry an ESP template: outgoing packets are encrypted with the
/// currently active key and incoming ones are only accepted when they arrived
/// through the tunnel (`level required`).
///
/// Both the `in` and the `fwd` direction are written: `in` covers packets that
/// are delivered locally, `fwd` the ones that are routed onwards to the TAP of
/// the VM, which is the normal case here.
///
/// # Arguments
/// * `conn` - The connection whose policies should be (re)written
/// * `local_gateway_ip` - Underlay address of this gateway
///
/// # Returns
/// `Ok(())` when the kernel accepted all policies of the connection
pub fn apply_connection_policies(
    conn: &Connection,
    local_gateway_ip: Ipv4Addr,
) -> Result<(), String> {
    let local_gateway_ip = local_gateway_ip.to_string();
    let peer_gateway_ip = conn.peer_gateway_ip.to_string();
    let local_sel = format!("{}/32", conn.local_ip);
    let remote_sel = format!("{}/32", conn.remote_ip);

    // Outbound: pin the policy to the key that is currently active, so that
    // installing another egress key is what switches a connection over.
    let spi = format!("0x{:08x}", conn.active_egress_spi);
    run_ip(&[
        "xfrm",
        "policy",
        "update",
        "src",
        &local_sel,
        "dst",
        &remote_sel,
        "dir",
        "out",
        "tmpl",
        "src",
        &local_gateway_ip,
        "dst",
        &peer_gateway_ip,
        "proto",
        "esp",
        "spi",
        &spi,
        "mode",
        "tunnel",
    ])?;

    // Inbound: demand ESP, so unprotected packets of this connection are dropped.
    for dir in ["in", "fwd"] {
        run_ip(&[
            "xfrm",
            "policy",
            "update",
            "src",
            &remote_sel,
            "dst",
            &local_sel,
            "dir",
            dir,
            "tmpl",
            "src",
            &peer_gateway_ip,
            "dst",
            &local_gateway_ip,
            "proto",
            "esp",
            "mode",
            "tunnel",
            "level",
            "required",
        ])?;
    }
    Ok(())
}

/// Installs one Security Association, replacing an existing one with the same SPI.
///
/// `ip xfrm state` has no atomic replace: `add` refuses to overwrite and
/// `update` refuses to create. Re-installing a key therefore means removing the
/// old state first, which also makes the endpoint idempotent - handing the same
/// SPI a new key simply rewrites it.
///
/// # Arguments
/// * `src` - Underlay address the ESP packets originate from
/// * `dst` - Underlay address the ESP packets are sent to
/// * `spi` - Security Parameter Index identifying this key on the wire
/// * `key` - Normalised AES-256-GCM key material (32 byte key + 4 byte salt)
/// * `sel_src` - Selector source, the VM address traffic comes from
/// * `sel_dst` - Selector destination, the VM address traffic goes to
///
/// # Returns
/// `Ok(())` when the kernel accepted the Security Association, otherwise an
/// error message with the key masked
pub fn install_sa(
    src: Ipv4Addr,
    dst: Ipv4Addr,
    spi: &str,
    key: &Secret,
    sel_src: &str,
    sel_dst: &str,
) -> Result<(), String> {
    let src = src.to_string();
    let dst = dst.to_string();

    // A leftover state under this SPI would make the add fail; it is the key we
    // are about to overwrite anyway.
    let _ = run_ip(&[
        "xfrm", "state", "delete", "src", &src, "dst", &dst, "proto", "esp", "spi", spi,
    ]);

    run_ip_with_secrets(
        &[
            "xfrm",
            "state",
            "add",
            "src",
            &src,
            "dst",
            &dst,
            "proto",
            "esp",
            "spi",
            spi,
            "reqid",
            "0",
            "mode",
            "tunnel",
            "aead",
            "rfc4106(gcm(aes))",
            key.reveal(),
            "128",
            "sel",
            "src",
            sel_src,
            "dst",
            sel_dst,
        ],
        &[key],
    )
}

/// Installs the fail-closed xfrm policies of an encrypted route.
///
/// Both the outgoing and the forwarded direction of the destination are blocked
/// with a low precedence policy. The per-connection policies installed together
/// with a key carry the default priority 0 and therefore win over these, so the
/// effect is: a VM pair that has keys talks encrypted, everything else towards
/// that destination is dropped instead of silently going out in the clear.
///
/// # Arguments
/// * `dest_ip` - The protected destination address
///
/// # Returns
/// `Ok(())` if both policies could be installed
pub fn install_block_policies(dest_ip: Ipv4Addr) -> Result<(), String> {
    let dest = format!("{}/32", dest_ip);
    run_ip(&[
        "xfrm", "policy", "update", "dst", &dest, "dir", "out", "action", "block", "priority",
        "4000",
    ])?;
    run_ip(&[
        "xfrm", "policy", "update", "src", &dest, "dir", "fwd", "action", "block", "priority",
        "4000",
    ])?;
    Ok(())
}

/// Removes the fail-closed xfrm policies of an encrypted route again.
///
/// # Arguments
/// * `dest_ip` - The destination the policies were installed for
///
/// # Returns
/// None. Errors are ignored: the policies may already be gone.
pub fn remove_block_policies(dest_ip: Ipv4Addr) {
    let dest = format!("{}/32", dest_ip);
    let _ = run_ip(&["xfrm", "policy", "delete", "dst", &dest, "dir", "out"]);
    let _ = run_ip(&["xfrm", "policy", "delete", "src", &dest, "dir", "fwd"]);
}

/// Removes one Security Association from the kernel.
///
/// # Arguments
/// * `src` - Underlay address the ESP packets of the SA originate from
/// * `dst` - Underlay address the ESP packets of the SA are sent to
/// * `spi` - Security Parameter Index of the SA
///
/// # Returns
/// `Ok(())` when the kernel removed the SA, otherwise the error of `ip`
pub fn remove_sa(src: Ipv4Addr, dst: Ipv4Addr, spi: u32) -> Result<(), String> {
    let (src, dst) = (src.to_string(), dst.to_string());
    let spi = format!("0x{:08x}", spi);
    run_ip(&[
        "xfrm", "state", "delete", "src", &src, "dst", &dst, "proto", "esp", "spi", &spi,
    ])
}

/// Reads the number of packets, which an SA processed so far.
///
/// # Arguments
/// * `src` - Source address of the SA
/// * `dst` - Destination address of the SA
/// * `spi` - SPI of the SA
///
/// # Returns
/// The number of packets, `None` if the kernel has no such SA (anymore), or the error of `ip`
pub fn sa_packet_count(src: Ipv4Addr, dst: Ipv4Addr, spi: u32) -> Result<Option<u64>, String> {
    let (src, dst) = (src.to_string(), dst.to_string());
    let spi = format!("0x{:08x}", spi);
    match run_ip_output(&[
        "-s", "xfrm", "state", "get", "src", &src, "dst", &dst, "proto", "esp", "spi", &spi,
    ]) {
        Ok(output) => parse_packet_count(&output).map(Some),
        // the kernel answers with ESRCH for an unknown SA
        Err(e) if e.contains("No such process") => Ok(None),
        Err(e) => Err(e),
    }
}

/// Reads the packet counter from the output of `ip -s xfrm state get`, which is the second
/// value of the `lifetime current:` block, like `  1234(bytes), 56(packets)`.
///
/// # Arguments
/// * `output` - The output of the command
///
/// # Returns
/// The number of packets, or an error, if the output has no counter
fn parse_packet_count(output: &str) -> Result<u64, String> {
    output
        .split_once("lifetime current:")
        .and_then(|(_, current)| {
            current
                .split_whitespace()
                .find_map(|token| token.strip_suffix("(packets)"))
        })
        .and_then(|count| count.parse().ok())
        .ok_or_else(|| "the output of the SA has no packet counter".to_string())
}

/// Removes the xfrm policies of a connection, which `apply_connection_policies` wrote.
///
/// Without them the traffic of the VM pair falls back to the fail-closed block policies of its
/// route, as long as the route exists.
///
/// # Arguments
/// * `conn` - The connection whose policies should be removed
///
/// # Returns
/// None. Errors are ignored: the policies may already be gone.
pub fn remove_connection_policies(conn: &Connection) {
    let local_sel = format!("{}/32", conn.local_ip);
    let remote_sel = format!("{}/32", conn.remote_ip);
    let _ = run_ip(&[
        "xfrm",
        "policy",
        "delete",
        "src",
        &local_sel,
        "dst",
        &remote_sel,
        "dir",
        "out",
    ]);
    for dir in ["in", "fwd"] {
        let _ = run_ip(&[
            "xfrm",
            "policy",
            "delete",
            "src",
            &remote_sel,
            "dst",
            &local_sel,
            "dir",
            dir,
        ]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Output of `ip -s xfrm state get` of iproute2 6.1 for an SA of the gateway
    const SA_OUTPUT: &str = "src 10.0.0.1 dst 10.0.0.2
\tproto esp spi 0x80001234(2147488308) reqid 0(0x00000000) mode tunnel
\treplay-window 0 seq 0x00000000 flag  (0x00000000)
\taead rfc4106(gcm(aes)) 0x0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f2021222324 (288 bits) 128
\tlifetime config:
\t  limit: soft (INF)(bytes), hard (INF)(bytes)
\t  limit: soft (INF)(packets), hard (INF)(packets)
\t  expire add: soft 0(sec), hard 0(sec)
\t  expire use: soft 0(sec), hard 0(sec)
\tlifetime current:
\t  1234(bytes), 56(packets)
\t  add 2026-11-12 10:00:00 use 2026-11-12 10:00:05
\tstats:
\t  replay-window 0 replay 0 failed 0
\tsel src 192.168.0.1/32 dst 192.168.0.2/32 proto 0 reqid 0
";

    #[test]
    fn the_packet_counter_is_read_from_the_current_lifetime() {
        assert_eq!(parse_packet_count(SA_OUTPUT), Ok(56));
        // the limits of the config block are no counters
        let without_current = SA_OUTPUT.split("\tlifetime current:").next().unwrap();
        assert!(parse_packet_count(without_current).is_err());
    }
}
