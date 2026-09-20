//! IPsec key and policy handling.
//!
//! The gateway does not implement any cryptography itself: it programs the
//! kernel's xfrm stack, which carries the AES-256-GCM transformation. This
//! module owns everything that touches Security Associations and policies, so
//! the rules about when traffic is protected live in exactly one place.

use std::net::Ipv4Addr;

use crate::core::models::Connection;
use crate::core::utils::{run_ip, run_ip_with_secrets};

use ainari_common::secret::Secret;

/// Normalises and validates AES-256-GCM key material coming from the API.
///
/// `rfc4106(gcm(aes))` expects the 32 byte cipher key immediately followed by
/// the 4 byte salt, so exactly 36 bytes have to be provided. The value is
/// returned in the `0x...` form expected by iproute2.
///
/// # Arguments
/// * `key` - Hex encoded key material, with or without a `0x` prefix
///
/// # Returns
/// A `Result` with the normalised key, or a message describing the problem
pub fn normalize_key(key: &Secret) -> Result<Secret, String> {
    let hex = key
        .reveal()
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if hex.len() != 72 {
        return Err(format!(
            "AES-256-GCM needs 36 bytes (32 byte key + 4 byte salt) = 72 hex digits, got {}",
            hex.len()
        ));
    }
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("key must be hex encoded".to_string());
    }
    Ok(Secret::from(format!("0x{}", hex)))
}

/// Writes the xfrm policies of a connection according to its current state.
///
/// This is the single place that decides whether a connection is protected. With
/// the encryption switched on, the policies carry an ESP template: outgoing
/// packets are encrypted with the currently active key and incoming ones are
/// only accepted when they arrived through the tunnel (`level required`). With
/// the encryption switched off the very same policies become plain allow rules
/// without a template, which is what makes the traffic travel unencrypted even
/// though the keys are still sitting in the kernel.
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

    if !conn.enabled {
        // No template at all: nothing is applied on the way out and nothing is
        // demanded on the way in. The keys stay installed and unused.
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
            "action",
            "allow",
        ])?;
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
                "action",
                "allow",
            ])?;
        }
        return Ok(());
    }

    // Outbound: pin the policy to the key that is currently active, so that
    // installing another egress key is what switches a connection over.
    let spi = conn.active_egress_spi.map(|spi| format!("0x{:08x}", spi));
    let mut out_args: Vec<&str> = vec![
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
    ];
    if let Some(spi) = spi.as_deref() {
        out_args.extend_from_slice(&["spi", spi]);
    }
    out_args.extend_from_slice(&["mode", "tunnel"]);
    run_ip(&out_args)?;

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
