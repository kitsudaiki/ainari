// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Installs the keys of the networks into the kernel of the gateway.
//!
//! Everything, which touches the xfrm-state and the policies of the gateway, is in here, so
//! the order of the changes, which never leaves a connection without a matching key, lives
//! in one place.

use crate::config::CONFIG;
use crate::core::crypto::{
    apply_connection_policies, install_sa, remove_connection_policies, remove_sa,
};
use crate::core::ebpf_interface::EBPFInterface;
use crate::core::mls_key_exchange::keys::{
    DesiredConnection, DesiredSa, derive_sa, desired_connections, desired_sas,
};
use crate::core::mls_key_exchange::network_keys::{EpochId, NetworkKeys};
use crate::core::models::{Connection, CryptoKey};
use crate::core::utils::{bind_connection_to_table, get_local_ip, unbind_connection_from_table};
use ainari_api_structs::network_crypto_structs::CryptoDirection;
use openmls::prelude::*;
use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;

/// Builds the id, under which the gateway-state holds the policies of a connection.
fn connection_id(vni: u32, conn: &DesiredConnection) -> String {
    format!("{}:{}->{}", vni, conn.local_ip, conn.remote_ip)
}

/// Checks, if the keys of an epoch are in place for all connections of a pair in one direction:
/// the incoming keys are installed, or the outbound policies point at the outgoing keys.
///
/// # Arguments
/// * `ebpf_interf` - The gateway state
/// * `keys` - The epochs of the network
/// * `crypto` - Crypto-provider of the MLS-client
/// * `vni` - Tenant of the network
/// * `peer` - The peer of the pair
/// * `id` - The epoch
/// * `direction` - The direction
/// * `local_gateway_ip` - Underlay address of this gateway
///
/// # Returns
/// `true` if all keys are in place, also if the pair has no connection
#[allow(clippy::too_many_arguments)]
pub fn pair_installed(
    ebpf_interf: &EBPFInterface,
    keys: &NetworkKeys,
    crypto: &impl OpenMlsCrypto,
    vni: u32,
    peer: Ipv4Addr,
    id: EpochId,
    direction: CryptoDirection,
    local_gateway_ip: Ipv4Addr,
) -> bool {
    let Some(base) = keys.bases.get(&id) else {
        return false;
    };
    desired_connections(&ebpf_interf.routes, &ebpf_interf.taps, vni)
        .iter()
        .filter(|conn| conn.peer_gateway_ip == peer)
        .all(|conn| {
            let local = (conn.local_ip, local_gateway_ip);
            let remote = (conn.remote_ip, conn.peer_gateway_ip);
            match direction {
                CryptoDirection::Ingress => derive_sa(base, crypto, vni, remote, local)
                    .is_ok_and(|sa| ebpf_interf.crypto_keys.contains_key(&(direction, sa.spi))),
                CryptoDirection::Egress => {
                    derive_sa(base, crypto, vni, local, remote).is_ok_and(|sa| {
                        ebpf_interf
                            .connections
                            .get(&connection_id(vni, conn))
                            .is_some_and(|existing| existing.active_egress_spi == sa.spi)
                    })
                }
            }
        })
}

/// Selects the installed keys of a network, which are not needed anymore.
///
/// # Arguments
/// * `keys` - All installed keys of the gateway
/// * `vni` - Tenant of the network
/// * `keep` - The keys, which have to stay, by direction and SPI
///
/// # Returns
/// The keys to remove, by direction and SPI
pub fn stale_keys(
    keys: &HashMap<(CryptoDirection, u32), CryptoKey>,
    vni: u32,
    keep: &HashSet<(CryptoDirection, u32)>,
) -> Vec<(CryptoDirection, u32)> {
    let mut stale: Vec<(CryptoDirection, u32)> = keys
        .values()
        .filter(|key| key.vni == vni && !keep.contains(&(key.direction, key.spi)))
        .map(|key| (key.direction, key.spi))
        .collect();
    stale.sort();
    stale
}

/// Brings the keys of a network in line with its recorded epochs and its routes.
///
/// The changes are made in the order, which never leaves a connection without a matching key on
/// both ends: first all missing incoming keys are installed, then the outgoing keys of the active
/// epochs of the pairs are installed and the outbound policies are switched over to them, and
/// only at the end the keys, which are not needed anymore, are removed. An outgoing key, whose
/// successor couldn't be installed, is kept, because the policy of its connection still points at
/// it.
///
/// Without recorded epochs every key of the network is removed, which leaves the encrypted routes
/// of the network with their fail-closed block policies.
///
/// A single connection, which fails, doesn't stop the others.
///
/// # Arguments
/// * `ebpf_interf` - The locked gateway state
/// * `keys` - The epochs of the network, or `None` if the gateway is not a member of its group
/// * `crypto` - Crypto-provider of the MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// `Ok(())` if all keys are in place, otherwise the collected errors
pub fn apply_network_keys(
    ebpf_interf: &mut EBPFInterface,
    keys: Option<&NetworkKeys>,
    crypto: &impl OpenMlsCrypto,
    vni: u32,
) -> Result<(), String> {
    let connections = match keys {
        Some(_) => desired_connections(&ebpf_interf.routes, &ebpf_interf.taps, vni),
        None => Vec::new(),
    };
    let has_state = ebpf_interf.crypto_keys.values().any(|key| key.vni == vni)
        || ebpf_interf.connections.values().any(|conn| conn.vni == vni);
    if connections.is_empty() && !has_state {
        return Ok(());
    }

    let local_gateway_ip = get_local_ip(&CONFIG.network.underlay_iface)
        .ok_or_else(|| format!("No underlay address on {}", CONFIG.network.underlay_iface))?;
    let sas = match keys {
        Some(keys) => desired_sas(keys, crypto, vni, &connections, local_gateway_ip)?,
        None => Vec::new(),
    };

    let errors = install_desired(ebpf_interf, vni, &sas, local_gateway_ip);

    // the outgoing key, which a policy points at, stays in any case, so a failed switch doesn't
    // leave the connection without its key
    let mut keep: HashSet<(CryptoDirection, u32)> = sas
        .iter()
        .map(|desired| (desired.direction, desired.sa.spi))
        .collect();
    keep.extend(
        ebpf_interf
            .connections
            .values()
            .filter(|conn| conn.vni == vni)
            .map(|conn| (CryptoDirection::Egress, conn.active_egress_spi)),
    );
    remove_stale_connections(ebpf_interf, vni, &connections, &mut keep);
    remove_stale_keys(ebpf_interf, vni, &keep, local_gateway_ip);

    log::debug!(
        "Applied the keys of tenant {vni} for {} connection(s), pairs {:?}",
        connections.len(),
        keys.map(|keys| &keys.pairs)
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Installs the desired keys of a network: first all incoming keys, so the peers can already
/// send with them, then the outgoing keys together with the switch of the outbound policies.
///
/// # Arguments
/// * `ebpf_interf` - The locked gateway state
/// * `vni` - Tenant of the network
/// * `sas` - The desired keys
/// * `local_gateway_ip` - Underlay address of this gateway
///
/// # Returns
/// The errors of the keys, which couldn't be installed
fn install_desired(
    ebpf_interf: &mut EBPFInterface,
    vni: u32,
    sas: &[DesiredSa],
    local_gateway_ip: Ipv4Addr,
) -> Vec<String> {
    let mut errors = Vec::new();
    for desired in sas
        .iter()
        .filter(|desired| desired.direction == CryptoDirection::Ingress)
    {
        if let Err(e) = install_key(ebpf_interf, vni, desired, local_gateway_ip) {
            errors.push(e);
        }
    }
    for desired in sas
        .iter()
        .filter(|desired| desired.direction == CryptoDirection::Egress)
    {
        let result = install_key(ebpf_interf, vni, desired, local_gateway_ip)
            .and_then(|()| activate_egress_key(ebpf_interf, vni, desired, local_gateway_ip));
        if let Err(e) = result {
            errors.push(e);
        }
    }
    errors
}

/// Removes the policies of the connections of a network, which are not desired anymore, so their
/// outgoing keys can go as well.
///
/// # Arguments
/// * `ebpf_interf` - The locked gateway state
/// * `vni` - Tenant of the network
/// * `connections` - The desired connections
/// * `keep` - The keys to keep, which loses the outgoing keys of the removed connections
fn remove_stale_connections(
    ebpf_interf: &mut EBPFInterface,
    vni: u32,
    connections: &[DesiredConnection],
    keep: &mut HashSet<(CryptoDirection, u32)>,
) {
    let desired_pairs: HashSet<(Ipv4Addr, Ipv4Addr)> = connections
        .iter()
        .map(|conn| (conn.local_ip, conn.remote_ip))
        .collect();
    let stale_connections: Vec<String> = ebpf_interf
        .connections
        .iter()
        .filter(|(_, conn)| {
            conn.vni == vni && !desired_pairs.contains(&(conn.local_ip, conn.remote_ip))
        })
        .map(|(conn_id, _)| conn_id.clone())
        .collect();
    for conn_id in stale_connections {
        if let Some(conn) = ebpf_interf.connections.remove(&conn_id) {
            remove_connection_policies(&conn);
            unbind_connection_from_table(
                conn.local_ip,
                conn.remote_ip,
                &CONFIG.network.tenant_table(conn.vni),
            );
            keep.remove(&(CryptoDirection::Egress, conn.active_egress_spi));
        }
    }
}

/// Removes the installed keys of a network, which are not kept.
///
/// # Arguments
/// * `ebpf_interf` - The locked gateway state
/// * `vni` - Tenant of the network
/// * `keep` - The keys to keep, by direction and SPI
/// * `local_gateway_ip` - Underlay address of this gateway
fn remove_stale_keys(
    ebpf_interf: &mut EBPFInterface,
    vni: u32,
    keep: &HashSet<(CryptoDirection, u32)>,
    local_gateway_ip: Ipv4Addr,
) {
    for (direction, spi) in stale_keys(&ebpf_interf.crypto_keys, vni, keep) {
        let Some(key) = ebpf_interf.crypto_keys.remove(&(direction, spi)) else {
            continue;
        };
        let (src, dst) = match direction {
            CryptoDirection::Egress => (local_gateway_ip, key.peer_gateway_ip),
            CryptoDirection::Ingress => (key.peer_gateway_ip, local_gateway_ip),
        };
        // the kernel may have dropped it already, for example with a restart of the host
        match remove_sa(src, dst, spi) {
            Ok(()) => log::debug!(
                "Removed {direction} key spi 0x{spi:08x} of epoch {} of tenant {vni}",
                key.mls_epoch
            ),
            Err(e) => log::warn!("Failed to remove {direction} key spi 0x{spi:08x}: {e}"),
        }
    }
}

/// Installs one key, if it isn't installed already.
///
/// # Arguments
/// * `ebpf_interf` - The locked gateway state
/// * `vni` - Tenant of the network
/// * `desired` - The key
/// * `local_gateway_ip` - Underlay address of this gateway
///
/// # Returns
/// `Ok(())` once the key is installed, or an error, if the kernel refused it or its SPI is used
/// by another connection
fn install_key(
    ebpf_interf: &mut EBPFInterface,
    vni: u32,
    desired: &DesiredSa,
    local_gateway_ip: Ipv4Addr,
) -> Result<(), String> {
    let conn = &desired.conn;
    let spi = desired.sa.spi;

    if let Some(existing) = ebpf_interf.crypto_keys.get(&(desired.direction, spi)) {
        let same_connection = existing.vni == vni
            && existing.local_ip == conn.local_ip
            && existing.remote_ip == conn.remote_ip
            && existing.peer_gateway_ip == conn.peer_gateway_ip;
        if same_connection {
            return Ok(());
        }
        return Err(format!(
            "SPI 0x{spi:08x} of {} -> {} is already used by {} -> {} in tenant {}",
            conn.local_ip, conn.remote_ip, existing.local_ip, existing.remote_ip, existing.vni
        ));
    }

    let local_sel = format!("{}/32", conn.local_ip);
    let remote_sel = format!("{}/32", conn.remote_ip);
    let spi_hex = format!("0x{spi:08x}");
    match desired.direction {
        CryptoDirection::Egress => install_sa(
            local_gateway_ip,
            conn.peer_gateway_ip,
            &spi_hex,
            &desired.sa.key,
            &local_sel,
            &remote_sel,
        )?,
        CryptoDirection::Ingress => install_sa(
            conn.peer_gateway_ip,
            local_gateway_ip,
            &spi_hex,
            &desired.sa.key,
            &remote_sel,
            &local_sel,
        )?,
    }

    ebpf_interf.crypto_keys.insert(
        (desired.direction, spi),
        CryptoKey {
            direction: desired.direction,
            vni,
            local_ip: conn.local_ip,
            remote_ip: conn.remote_ip,
            peer_gateway_ip: conn.peer_gateway_ip,
            spi,
            mls_epoch: desired.epoch,
        },
    );
    Ok(())
}

/// Points the outbound policy of a connection at its outgoing key of the active epoch.
///
/// The policies are only rewritten, if the connection moved to another key or peer, so a refresh
/// without a change doesn't touch the kernel.
///
/// # Arguments
/// * `ebpf_interf` - The locked gateway state
/// * `vni` - Tenant of the network
/// * `desired` - The outgoing key of the connection
/// * `local_gateway_ip` - Underlay address of this gateway
///
/// # Returns
/// `Ok(())` once the policies point at the key
fn activate_egress_key(
    ebpf_interf: &mut EBPFInterface,
    vni: u32,
    desired: &DesiredSa,
    local_gateway_ip: Ipv4Addr,
) -> Result<(), String> {
    let conn = &desired.conn;
    let conn_id = connection_id(vni, conn);
    let connection = Connection {
        vni,
        local_ip: conn.local_ip,
        remote_ip: conn.remote_ip,
        peer_gateway_ip: conn.peer_gateway_ip,
        active_egress_spi: desired.sa.spi,
    };

    let unchanged = ebpf_interf
        .connections
        .get(&conn_id)
        .is_some_and(|existing| {
            existing.active_egress_spi == connection.active_egress_spi
                && existing.peer_gateway_ip == connection.peer_gateway_ip
        });
    if !unchanged {
        // a new connection needs its rules first, so the first packet, which it decrypts, finds
        // the VM instead of the default route
        if !ebpf_interf.connections.contains_key(&conn_id) {
            bind_connection_to_table(
                conn.local_ip,
                conn.remote_ip,
                &CONFIG.network.tenant_table(vni),
            )?;
        }
        apply_connection_policies(&connection, local_gateway_ip)?;
        ebpf_interf.connections.insert(conn_id, connection);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::test_support::{gateway, ip};

    fn key(direction: CryptoDirection, vni: u32, spi: u32) -> CryptoKey {
        CryptoKey {
            direction,
            vni,
            local_ip: ip(1),
            remote_ip: ip(2),
            peer_gateway_ip: gateway(2),
            spi,
            mls_epoch: 1,
        }
    }

    #[test]
    fn only_the_keys_of_the_tenant_which_are_not_kept_are_stale() {
        let mut keys = HashMap::new();
        for key in [
            key(CryptoDirection::Egress, 5, 1),
            key(CryptoDirection::Ingress, 5, 2),
            key(CryptoDirection::Ingress, 5, 3),
            key(CryptoDirection::Ingress, 6, 4),
        ] {
            keys.insert((key.direction, key.spi), key);
        }
        let keep: HashSet<_> = [(CryptoDirection::Egress, 1), (CryptoDirection::Ingress, 2)]
            .into_iter()
            .collect();

        assert_eq!(
            stale_keys(&keys, 5, &keep),
            vec![(CryptoDirection::Ingress, 3)]
        );
        assert_eq!(
            stale_keys(&keys, 6, &HashSet::new()),
            vec![(CryptoDirection::Ingress, 4)]
        );
    }
}
