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

//! IPsec keys, which are derived from the MLS-group of a network.
//!
//! All members of a group share the same epoch-secret, so every gateway can derive the keys of a
//! connection on its own, without any key ever being sent over the wire. A base secret is exported
//! from every epoch of the group (see `NetworkKeys`) and the key, the salt and the SPI of the
//! traffic from one VM to another are derived from it with the tenant, the two VMs and their two
//! gateways as context. The sending gateway installs it as its egress key, the receiving gateway
//! derives exactly the same one as its ingress key.
//!
//! # Key-rotation without packet loss
//!
//! Every change of the group moves it into a new epoch, which rotates all keys. Both ends of a
//! connection always have to agree: a sender may only use a key, which the receiver already has,
//! and a receiver may only drop a key, which no sender uses anymore. So the keys of a new epoch
//! are rolled out in three steps, which the control plane runs on all gateways of a network one
//! after another:
//!
//! 1. **record**: the incoming keys of the new epoch are installed next to the old ones, the
//!    outgoing traffic still uses the old epoch
//! 2. **activate**: once every gateway of the network has recorded the new epoch, the outgoing
//!    policy of every connection is switched to the key of the new epoch. The switch of a policy
//!    is atomic and the peer can receive with both epochs at this point.
//! 3. **retire**: once every gateway has activated the new epoch, the incoming keys of the old
//!    epochs are removed.

use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;

use openmls::prelude::*;
use uuid::Uuid;

use crate::config::CONFIG;
use crate::core::crypto::{
    apply_connection_policies, install_sa, remove_connection_policies, remove_sa,
};
use crate::core::ebpf_interface::EBPFInterface;
use crate::core::mls_key_exchange::state::NetworkKeys;
use crate::core::models::{Connection, CryptoKey, Route, TapInfo};
use crate::core::utils::{bind_connection_to_table, get_local_ip, unbind_connection_from_table};

use ainari_api_structs::network_crypto_structs::CryptoDirection;
use ainari_common::secret::Secret;

/// Label of the MLS-exporter for the base secret of an epoch
const BASE_EXPORTER_LABEL: &str = "ainari ipsec base v1";

/// Length of the base secret of an epoch
const BASE_LENGTH: usize = 32;

/// Label, which prefixes the context of every key derived from a base secret
const SA_INFO_LABEL: &str = "ainari ipsec sa v1";

/// Length of the AES-256-GCM key material: 32 byte key + 4 byte salt
const SA_KEY_LENGTH: usize = 36;

/// Bit, which is set in every SPI derived from a group. It keeps the derived SPIs out of the
/// range 1-255, which is reserved.
pub const MLS_SPI_FLAG: u32 = 0x8000_0000;

/// One direction of a connection, as derived from the base secret of an epoch
pub struct DerivedSa {
    pub spi: u32,
    /// Key material in the `0x...` form expected by iproute2
    pub key: Secret,
}

/// A VM-pair, whose traffic has to be protected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DesiredConnection {
    /// VM behind this gateway
    pub local_ip: Ipv4Addr,
    /// VM behind the peer gateway
    pub remote_ip: Ipv4Addr,
    /// Underlay-address of the gateway of the remote VM
    pub peer_gateway_ip: Ipv4Addr,
}

/// A key, which has to be installed on this gateway
pub struct DesiredSa {
    pub direction: CryptoDirection,
    pub epoch: u64,
    pub conn: DesiredConnection,
    pub sa: DerivedSa,
}

/// Exports the base secret of the current epoch of a group.
///
/// # Arguments
/// * `group` - The group of the network
/// * `crypto` - Crypto-provider of the MLS-client
///
/// # Returns
/// The base secret, or the error of the exporter
pub fn derive_base(group: &MlsGroup, crypto: &impl OpenMlsCrypto) -> Result<Vec<u8>, String> {
    group
        .export_secret(crypto, BASE_EXPORTER_LABEL, b"", BASE_LENGTH)
        .map_err(|e| format!("Failed to export the base secret: {e}"))
}

/// Derives the key of the traffic from one VM to another from the base secret of an epoch.
///
/// The two gateways are part of the context, so a VM, which moves to another host, gets new keys
/// and new SPIs, instead of an SA, which would have to be replaced under the same SPI.
///
/// # Arguments
/// * `base` - Base secret of the epoch
/// * `crypto` - Crypto-provider of the MLS-client
/// * `vni` - Tenant of the network
/// * `src` - The VM, which sends the traffic, together with its gateway
/// * `dst` - The VM, which receives the traffic, together with its gateway
///
/// # Returns
/// The SPI and the key material, or the error of the key-derivation
pub fn derive_sa(
    base: &[u8],
    crypto: &impl OpenMlsCrypto,
    vni: u32,
    src: (Ipv4Addr, Ipv4Addr),
    dst: (Ipv4Addr, Ipv4Addr),
) -> Result<DerivedSa, String> {
    let info = format!(
        "{SA_INFO_LABEL}:{vni}:{}@{}->{}@{}",
        src.0, src.1, dst.0, dst.1
    );
    let okm = crypto
        .hkdf_expand(HashType::Sha2_256, base, info.as_bytes(), SA_KEY_LENGTH + 4)
        .map_err(|e| format!("Failed to derive the key of {} -> {}: {e:?}", src.0, dst.0))?;
    let okm = okm.as_slice();

    let key: String = okm[..SA_KEY_LENGTH]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut spi_bytes = [0u8; 4];
    spi_bytes.copy_from_slice(&okm[SA_KEY_LENGTH..]);

    Ok(DerivedSa {
        spi: u32::from_be_bytes(spi_bytes) | MLS_SPI_FLAG,
        key: Secret::from(format!("0x{key}")),
    })
}

/// Lists the VM-pairs of a network, which have to be protected on this gateway.
///
/// Each VM behind this gateway - the target of a local route onto a TAP of the tenant - is paired
/// with each VM, which is reached over an encrypted route of the same tenant.
///
/// # Arguments
/// * `routes` - All routes of the gateway
/// * `taps` - All TAP devices of the gateway
/// * `vni` - Tenant of the network
///
/// # Returns
/// The connections, ordered by their addresses
pub fn desired_connections(
    routes: &HashMap<Uuid, Route>,
    taps: &HashMap<String, TapInfo>,
    vni: u32,
) -> Vec<DesiredConnection> {
    let mut local_ips: Vec<Ipv4Addr> = routes
        .values()
        .filter(|route| route.vni == vni && route.gateway_ip.is_none())
        .filter(|route| {
            taps.get(&route.target_iface)
                .is_some_and(|tap| tap.vni == vni)
        })
        .map(|route| route.dest_ip)
        .collect();
    local_ips.sort();
    local_ips.dedup();

    let mut remotes: Vec<(Ipv4Addr, Ipv4Addr)> = routes
        .values()
        .filter(|route| route.vni == vni && route.encrypted)
        .filter_map(|route| {
            route
                .gateway_ip
                .map(|gateway_ip| (route.dest_ip, gateway_ip))
        })
        .collect();
    remotes.sort();
    remotes.dedup_by_key(|(remote_ip, _)| *remote_ip);

    let mut connections = Vec::with_capacity(local_ips.len() * remotes.len());
    for local_ip in &local_ips {
        for (remote_ip, peer_gateway_ip) in &remotes {
            connections.push(DesiredConnection {
                local_ip: *local_ip,
                remote_ip: *remote_ip,
                peer_gateway_ip: *peer_gateway_ip,
            });
        }
    }
    connections
}

/// Derives all keys, which have to be installed on this gateway for the connections of a network.
///
/// The incoming keys are derived for every recorded epoch, so the peers can send with any of
/// them. The outgoing keys are only derived for the active epoch, or for the active epoch of a
/// replaced group, as long as the new group didn't switch yet.
///
/// # Arguments
/// * `keys` - The epochs of the network
/// * `crypto` - Crypto-provider of the MLS-client
/// * `vni` - Tenant of the network
/// * `connections` - The connections of the network on this gateway
/// * `local_gateway_ip` - Underlay address of this gateway
///
/// # Returns
/// The keys, or the error of the key-derivation
pub fn desired_sas(
    keys: &NetworkKeys,
    crypto: &impl OpenMlsCrypto,
    vni: u32,
    connections: &[DesiredConnection],
    local_gateway_ip: Ipv4Addr,
) -> Result<Vec<DesiredSa>, String> {
    let (egress_epoch, egress_base) = keys.egress().ok_or_else(|| {
        format!(
            "Active epoch {} of tenant {vni} has no key material",
            keys.active_epoch
        )
    })?;
    let ingress = keys.ingress();

    let mut sas = Vec::new();
    for conn in connections {
        let local = (conn.local_ip, local_gateway_ip);
        let remote = (conn.remote_ip, conn.peer_gateway_ip);

        for (epoch, base) in &ingress {
            sas.push(DesiredSa {
                direction: CryptoDirection::Ingress,
                epoch: *epoch,
                conn: *conn,
                sa: derive_sa(base, crypto, vni, remote, local)?,
            });
        }
        sas.push(DesiredSa {
            direction: CryptoDirection::Egress,
            epoch: egress_epoch,
            conn: *conn,
            sa: derive_sa(egress_base, crypto, vni, local, remote)?,
        });
    }
    Ok(sas)
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
/// epoch are installed and the outbound policies are switched over to them, and only at the end
/// the keys, which are not needed anymore, are removed. An outgoing key, whose successor couldn't
/// be installed, is kept, because the policy of its connection still points at it.
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
    let mut errors = Vec::new();

    // 1. incoming keys first, so the peers can already send with them
    for desired in sas
        .iter()
        .filter(|desired| desired.direction == CryptoDirection::Ingress)
    {
        if let Err(e) = install_key(ebpf_interf, vni, desired, local_gateway_ip) {
            errors.push(e);
        }
    }

    // 2. outgoing keys of the active epoch, together with the switch of the outbound policies
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

    // 3. remove what is not needed anymore. The outgoing key, which a policy points at, stays in
    //    any case, so a failed switch doesn't leave the connection without its key.
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

    for (direction, spi) in stale_keys(&ebpf_interf.crypto_keys, vni, &keep) {
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

    log::debug!(
        "Applied the keys of tenant {vni} for {} connection(s), active epoch {:?}, epochs {:?}",
        connections.len(),
        keys.map(|keys| keys.active_epoch),
        keys.map(|keys| keys.bases.keys().collect::<Vec<_>>())
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
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
    let conn_id = format!("{}:{}->{}", vni, conn.local_ip, conn.remote_ip);
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
    use std::collections::BTreeMap;

    use openmls_rust_crypto::RustCrypto;

    use crate::core::mls_key_exchange::group::create_group;
    use crate::core::mls_key_exchange::group::tests::{invite, new_gateway};

    fn ip(last: u8) -> Ipv4Addr {
        Ipv4Addr::new(192, 168, 0, last)
    }

    fn gateway(last: u8) -> Ipv4Addr {
        Ipv4Addr::new(10, 0, 0, last)
    }

    fn route(vni: u32, dest_ip: Ipv4Addr, target_iface: &str, gateway_last: Option<u8>) -> Route {
        Route {
            uuid: Uuid::new_v4(),
            vni,
            dest_ip,
            target_iface: target_iface.to_string(),
            gateway_ip: gateway_last.map(gateway),
            next_hop_ip: None,
            next_hop_mac: None,
            encrypted: gateway_last.is_some(),
        }
    }

    fn tap(vni: u32) -> TapInfo {
        TapInfo {
            vni,
            tap_mac: [0; 6],
            vm_mac: None,
        }
    }

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

    /// One gateway of the rotation-tests with one VM, which talks to the VM of the other one
    struct TestGateway {
        conn: DesiredConnection,
        local_gateway_ip: Ipv4Addr,
        keys: NetworkKeys,
    }

    impl TestGateway {
        fn new(local_vm: u8, local_gateway: u8, remote_vm: u8, remote_gateway: u8) -> Self {
            let mut bases = BTreeMap::new();
            bases.insert(1, vec![1u8; BASE_LENGTH]);
            TestGateway {
                conn: DesiredConnection {
                    local_ip: ip(local_vm),
                    remote_ip: ip(remote_vm),
                    peer_gateway_ip: gateway(remote_gateway),
                },
                local_gateway_ip: gateway(local_gateway),
                keys: NetworkKeys {
                    active_epoch: 1,
                    bases,
                    replaced: None,
                },
            }
        }

        fn spis(&self, direction: CryptoDirection) -> HashSet<u32> {
            desired_sas(
                &self.keys,
                &RustCrypto::default(),
                5,
                &[self.conn],
                self.local_gateway_ip,
            )
            .unwrap()
            .into_iter()
            .filter(|desired| desired.direction == direction)
            .map(|desired| desired.sa.spi)
            .collect()
        }

        fn record(&mut self, epoch: u64) {
            self.keys
                .bases
                .insert(epoch, vec![epoch as u8; BASE_LENGTH]);
        }

        fn activate(&mut self, epoch: u64) {
            self.keys.activate(epoch).unwrap();
        }

        fn retire(&mut self, epoch: u64) {
            self.keys.retire(epoch).unwrap();
        }

        /// Starts a new group with its own base secrets, like after izakaya lost its database
        fn replace(&mut self, epoch: u64) {
            let keys = std::mem::take(&mut self.keys);
            self.keys = keys.replaced_by(epoch, vec![100 + epoch as u8; BASE_LENGTH]);
        }

        fn record_new_group(&mut self, epoch: u64) {
            self.keys
                .bases
                .insert(epoch, vec![100 + epoch as u8; BASE_LENGTH]);
        }
    }

    /// Checks, that the receiver has the key, which the sender uses, in both directions.
    fn in_sync(a: &TestGateway, b: &TestGateway) -> bool {
        let a_egress = a.spis(CryptoDirection::Egress);
        let b_egress = b.spis(CryptoDirection::Egress);
        a_egress.len() == 1
            && b_egress.len() == 1
            && a_egress.is_subset(&b.spis(CryptoDirection::Ingress))
            && b_egress.is_subset(&a.spis(CryptoDirection::Ingress))
    }

    #[test]
    fn both_gateways_derive_the_same_key_for_one_direction() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        let a_base = derive_base(&a.groups[&5], a.provider.crypto()).unwrap();
        let b_base = derive_base(&b.groups[&5], b.provider.crypto()).unwrap();
        assert_eq!(a_base, b_base);

        let crypto = RustCrypto::default();
        let a_side = (ip(1), gateway(1));
        let b_side = (ip(2), gateway(2));
        let a_egress = derive_sa(&a_base, &crypto, 5, a_side, b_side).unwrap();
        let b_ingress = derive_sa(&b_base, &crypto, 5, a_side, b_side).unwrap();
        assert_eq!(a_egress.spi, b_ingress.spi);
        assert_eq!(a_egress.key.reveal(), b_ingress.key.reveal());
        assert_eq!(a_egress.key.reveal().len(), 2 + 2 * SA_KEY_LENGTH);
        assert_ne!(a_egress.spi & MLS_SPI_FLAG, 0);

        // the other direction, another tenant and another gateway get keys of their own
        let back = derive_sa(&a_base, &crypto, 5, b_side, a_side).unwrap();
        let other_tenant = derive_sa(&a_base, &crypto, 6, a_side, b_side).unwrap();
        let moved = derive_sa(&a_base, &crypto, 5, a_side, (ip(2), gateway(3))).unwrap();
        for other in [back, other_tenant, moved] {
            assert_ne!(other.key.reveal(), a_egress.key.reveal());
            assert_ne!(other.spi, a_egress.spi);
        }
    }

    #[test]
    fn a_new_epoch_brings_new_keys() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        let before = derive_base(&a.groups[&5], a.provider.crypto()).unwrap();

        invite(&mut a, &mut c, &mut [&mut b], 5);
        let after = derive_base(&a.groups[&5], a.provider.crypto()).unwrap();
        assert_ne!(before, after);
    }

    #[test]
    fn both_ends_of_a_connection_stay_in_sync_during_a_rotation() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        assert!(in_sync(&a, &b));

        // the gateways follow the change of the group one after another
        a.record(2);
        assert!(in_sync(&a, &b));
        b.record(2);
        assert!(in_sync(&a, &b));
        assert_eq!(a.spis(CryptoDirection::Ingress).len(), 2);

        // ... switch their outgoing traffic one after another
        a.activate(2);
        assert!(in_sync(&a, &b));
        b.activate(2);
        assert!(in_sync(&a, &b));

        // ... and drop the old epoch one after another
        a.retire(2);
        assert!(in_sync(&a, &b));
        b.retire(2);
        assert!(in_sync(&a, &b));
        assert_eq!(a.spis(CryptoDirection::Ingress).len(), 1);
    }

    #[test]
    fn an_early_activation_would_break_the_connection() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let b = TestGateway::new(2, 2, 1, 1);

        // b doesn't know epoch 2 yet, so it can't receive, what a sends with it
        a.record(2);
        a.activate(2);
        assert!(!in_sync(&a, &b));
    }

    #[test]
    fn several_epochs_can_be_activated_at_once() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);

        // a stale membership is removed and added again, which moves the group twice
        for epoch in [2, 3] {
            a.record(epoch);
            b.record(epoch);
            assert!(in_sync(&a, &b));
        }
        a.activate(3);
        assert!(in_sync(&a, &b));
        b.activate(3);
        a.retire(3);
        b.retire(3);
        assert!(in_sync(&a, &b));
        assert_eq!(b.keys.bases.keys().copied().collect::<Vec<_>>(), vec![3]);
    }

    #[test]
    fn local_vms_are_paired_with_the_encrypted_remote_vms_of_their_tenant() {
        let mut routes = HashMap::new();
        for route in [
            route(5, ip(1), "tap1", None),
            route(5, ip(2), "eth0", Some(2)),
            route(5, ip(3), "eth0", Some(3)),
            // another tenant, an unencrypted route and a route onto a TAP of another tenant
            route(6, ip(4), "eth0", Some(2)),
            Route {
                encrypted: false,
                ..route(5, ip(5), "eth0", Some(2))
            },
            route(5, ip(6), "tap6", None),
        ] {
            routes.insert(route.uuid, route);
        }
        let mut taps = HashMap::new();
        taps.insert("tap1".to_string(), tap(5));
        taps.insert("tap6".to_string(), tap(6));

        let connections = desired_connections(&routes, &taps, 5);
        assert_eq!(
            connections,
            vec![
                DesiredConnection {
                    local_ip: ip(1),
                    remote_ip: ip(2),
                    peer_gateway_ip: gateway(2),
                },
                DesiredConnection {
                    local_ip: ip(1),
                    remote_ip: ip(3),
                    peer_gateway_ip: gateway(3),
                },
            ]
        );
        assert!(desired_connections(&routes, &taps, 7).is_empty());
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

    #[test]
    fn a_replaced_group_keeps_both_ends_in_sync() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        assert!(in_sync(&a, &b));

        // izakaya lost its database: a starts a new group, b joins it later
        a.replace(0);
        assert!(in_sync(&a, &b));
        a.record_new_group(1);
        b.replace(1);
        assert!(in_sync(&a, &b));

        // the first round of the new group switches both over and drops the old group
        a.activate(1);
        assert!(in_sync(&a, &b));
        b.activate(1);
        assert!(in_sync(&a, &b));
        a.retire(1);
        b.retire(1);
        assert!(in_sync(&a, &b));
        assert!(a.keys.replaced.is_none());
        assert_eq!(a.spis(CryptoDirection::Ingress).len(), 1);
    }

    #[test]
    fn a_group_without_its_old_keys_would_break_the_connection() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let b = TestGateway::new(2, 2, 1, 1);

        // a new group, which drops the keys of the old one, sends with keys b doesn't know
        a.keys = NetworkKeys::new(0, vec![100u8; BASE_LENGTH]);
        assert!(!in_sync(&a, &b));
    }

    #[test]
    fn the_replaced_group_is_only_retired_after_the_switch() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        a.replace(0);
        a.record_new_group(1);
        assert!(a.keys.retire(1).is_err());
        a.activate(1);
        assert!(a.keys.retire(1).is_ok());
    }
}
