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
//! Every change of the group moves it into a new epoch, which rotates the keys. The keys are kept
//! per pair of this gateway and a peer gateway (see `PairKeys`). Both ends of a connection always
//! have to agree: a sender may only use a key, which the receiver already has, and a receiver may
//! only drop a key, which no sender uses anymore. So the two gateways of a pair roll the keys of a
//! new epoch out in three steps, which only depend on each other (see `pairs`):
//!
//! 1. the incoming keys of the new epoch are installed next to the old ones, and the peer is told
//!    with a `Ready`
//! 2. after the `Ready` of the peer, the outgoing policy of every connection of the pair is
//!    switched to the key of the new epoch. The switch of a policy is atomic and the peer can
//!    receive with both epochs at this point. The peer is told with a `Switched`.
//! 3. after the `Switched` of the peer, the incoming keys of the old epochs are removed, once no
//!    packet arrives with them anymore.

use crate::core::mls_key_exchange::network_keys::{EpochId, NetworkKeys};
use crate::core::models::{Route, TapInfo};
use ainari_api_structs::network_crypto_structs::CryptoDirection;
use ainari_common::secret::Secret;
use openmls::prelude::*;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use uuid::Uuid;

/// Label of the MLS-exporter for the base secret of an epoch
const BASE_EXPORTER_LABEL: &str = "ainari ipsec base v1";

/// Length of the base secret of an epoch
pub(crate) const BASE_LENGTH: usize = 32;

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
/// Every connection uses the keys of the pair with its peer gateway: the incoming keys of every
/// epoch of the pair, so the peer can send with any of them, the incoming keys of the retiring
/// epochs, which are still in use, and the outgoing key of the active epoch of the pair. A
/// connection without a pair, or whose pair didn't switch to an epoch yet, gets no outgoing key,
/// so the block policy of its route keeps it fail-closed.
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
    let base_of = |id: &EpochId| {
        keys.bases.get(id).ok_or_else(|| {
            format!(
                "Epoch {} of generation {} of tenant {vni} has no key material",
                id.1, id.0
            )
        })
    };

    let mut sas = Vec::new();
    for conn in connections {
        let Some(pair) = keys.pairs.get(&conn.peer_gateway_ip) else {
            continue;
        };
        let local = (conn.local_ip, local_gateway_ip);
        let remote = (conn.remote_ip, conn.peer_gateway_ip);

        for id in &pair.ingress {
            sas.push(DesiredSa {
                direction: CryptoDirection::Ingress,
                epoch: id.1,
                conn: *conn,
                sa: derive_sa(base_of(id)?, crypto, vni, remote, local)?,
            });
        }
        for (id, counters) in &pair.retiring {
            let sa = derive_sa(base_of(id)?, crypto, vni, remote, local)?;
            // the keys of a retiring epoch are all kept, until their counters were read
            if counters.is_empty() || counters.contains_key(&sa.spi) {
                sas.push(DesiredSa {
                    direction: CryptoDirection::Ingress,
                    epoch: id.1,
                    conn: *conn,
                    sa,
                });
            }
        }
        if let Some(id) = &pair.egress {
            sas.push(DesiredSa {
                direction: CryptoDirection::Egress,
                epoch: id.1,
                conn: *conn,
                sa: derive_sa(base_of(id)?, crypto, vni, local, remote)?,
            });
        }
    }
    Ok(sas)
}

/// Removes the incoming keys of the retiring epochs of a network, which no packet uses anymore.
///
/// The packet counter of every incoming key of a retiring epoch is read in every call. A key,
/// whose counter didn't change since the last call, is dropped, and an epoch, which has no keys
/// left, is forgotten. So a key is kept, as long as packets arrive with it, and an idle one is
/// gone with the next call. The caller removes the dropped keys from the kernel with
/// `apply_network_keys`.
///
/// # Arguments
/// * `keys` - The epochs of the network
/// * `crypto` - Crypto-provider of the MLS-client
/// * `vni` - Tenant of the network
/// * `connections` - The connections of the network on this gateway
/// * `local_gateway_ip` - Underlay address of this gateway
/// * `packet_count` - Reads the packet counter of an SA by source, destination and SPI, `None`
///   if the SA doesn't exist
///
/// # Returns
/// `true` if the state changed and has to be saved
pub fn remove_idle_keys(
    keys: &mut NetworkKeys,
    crypto: &impl OpenMlsCrypto,
    vni: u32,
    connections: &[DesiredConnection],
    local_gateway_ip: Ipv4Addr,
    packet_count: &mut impl FnMut(Ipv4Addr, Ipv4Addr, u32) -> Result<Option<u64>, String>,
) -> bool {
    let mut changed = false;
    for (peer, pair) in keys.pairs.iter_mut() {
        let mut done = Vec::new();
        for (id, counters) in pair.retiring.iter_mut() {
            let first_check = counters.is_empty();
            let spis = match (first_check, keys.bases.get(id)) {
                (false, _) => counters.keys().copied().collect(),
                (true, Some(base)) => {
                    incoming_spis(base, crypto, vni, connections, *peer, local_gateway_ip)
                }
                (true, None) => Vec::new(),
            };
            let mut read = |spi| packet_count(*peer, local_gateway_ip, spi);
            changed |= check_counters(counters, &spis, first_check, &mut read);
            if counters.is_empty() {
                done.push(*id);
            }
        }
        for id in done {
            log::debug!(
                "Removed the old incoming keys of epoch {id:?} from {peer} in tenant {vni}"
            );
            pair.retiring.remove(&id);
            changed = true;
        }
    }
    if changed {
        keys.drop_unused_bases();
    }
    changed
}

/// Derives the SPIs of the incoming keys of an epoch, which a pair has for its connections.
///
/// # Arguments
/// * `base` - Base secret of the epoch
/// * `crypto` - Crypto-provider of the MLS-client
/// * `vni` - Tenant of the network
/// * `connections` - The connections of the network on this gateway
/// * `peer` - The peer of the pair
/// * `local_gateway_ip` - Underlay address of this gateway
///
/// # Returns
/// The SPIs
fn incoming_spis(
    base: &[u8],
    crypto: &impl OpenMlsCrypto,
    vni: u32,
    connections: &[DesiredConnection],
    peer: Ipv4Addr,
    local_gateway_ip: Ipv4Addr,
) -> Vec<u32> {
    connections
        .iter()
        .filter(|conn| conn.peer_gateway_ip == peer)
        .filter_map(|conn| {
            let local = (conn.local_ip, local_gateway_ip);
            let remote = (conn.remote_ip, conn.peer_gateway_ip);
            derive_sa(base, crypto, vni, remote, local).ok()
        })
        .map(|sa| sa.spi)
        .collect()
}

/// Reads the packet counters of the incoming keys of a retiring epoch and drops every key, which
/// is idle since the last check or gone already. A key, whose counter can't be read, is kept.
///
/// # Arguments
/// * `counters` - The counters of the last check by SPI, which are updated
/// * `spis` - The keys to check
/// * `first_check` - The counters were never read before, so every key is kept this time
/// * `read` - Reads the counter of a key, `None` if the key doesn't exist
///
/// # Returns
/// `true` if a key was checked
fn check_counters(
    counters: &mut BTreeMap<u32, u64>,
    spis: &[u32],
    first_check: bool,
    read: &mut impl FnMut(u32) -> Result<Option<u64>, String>,
) -> bool {
    for spi in spis {
        match read(*spi) {
            Ok(Some(count)) if first_check || counters.get(spi) != Some(&count) => {
                counters.insert(*spi, count);
            }
            // idle since the last check, or gone already
            Ok(_) => {
                counters.remove(spi);
            }
            Err(e) => {
                log::warn!("Failed to read the counter of spi 0x{spi:08x}: {e}");
                // without a reading, an unread key is kept and checked again
                if first_check {
                    counters.insert(*spi, u64::MAX);
                }
            }
        }
    }
    !spis.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::group::create_group;
    use crate::core::mls_key_exchange::network_keys::NetworkKeys;

    use crate::core::mls_key_exchange::test_support::{
        TestGateway, gateway, in_sync, invite, ip, new_gateway,
    };
    use openmls_rust_crypto::RustCrypto;

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
    fn an_early_activation_would_break_the_connection() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let b = TestGateway::new(2, 2, 1, 1);

        // b doesn't know epoch 2 yet, so it can't receive, what a sends with it
        a.record(2);
        a.switch(2);
        assert!(!in_sync(&a, &b));
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
    fn a_group_without_its_old_keys_would_break_the_connection() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let b = TestGateway::new(2, 2, 1, 1);

        // a new group, which drops the keys of the old one, sends with keys b doesn't know
        a.keys = NetworkKeys::new(0, vec![100u8; BASE_LENGTH], &a.peers());
        assert!(!in_sync(&a, &b));
    }

    #[test]
    fn an_old_incoming_key_stays_as_long_as_packets_arrive_with_it() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        for gateway in [&mut a, &mut b] {
            gateway.record(2);
            gateway.switch(2);
        }
        let old_spi = *a
            .spis(CryptoDirection::Ingress)
            .difference(&b.spis(CryptoDirection::Egress))
            .next()
            .unwrap();
        a.retire_older(2);
        assert_eq!(a.spis(CryptoDirection::Ingress).len(), 2);

        // packets still arrive with the old key, so it stays
        let mut packets = 10;
        for _ in 0..3 {
            packets += 1;
            assert!(a.check_counters(&mut |src, dst, spi| {
                assert_eq!((src, dst, spi), (gateway(2), gateway(1), old_spi));
                Ok(Some(packets))
            }));
            assert!(a.spis(CryptoDirection::Ingress).contains(&old_spi));
            assert!(in_sync(&a, &b));
        }

        // the first check without a new packet removes it
        assert!(a.check_counters(&mut |_, _, _| Ok(Some(packets))));
        assert!(!a.spis(CryptoDirection::Ingress).contains(&old_spi));
        assert!(a.keys.pairs[&gateway(2)].retiring.is_empty());
        assert_eq!(
            a.keys.bases.keys().copied().collect::<Vec<_>>(),
            vec![(1, 2)]
        );
        // nothing is left to check
        assert!(!a.check_counters(&mut |_, _, _| panic!("no key is checked")));
    }

    #[test]
    fn an_old_incoming_key_which_is_gone_counts_as_removed() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        a.record(2);
        a.switch(2);
        a.retire_older(2);

        // for example after a restart of the host
        a.check_counters(&mut |_, _, _| Ok(None));
        assert!(a.keys.pairs[&gateway(2)].retiring.is_empty());
        assert_eq!(a.spis(CryptoDirection::Ingress).len(), 1);
    }

    #[test]
    fn an_old_incoming_key_without_counter_is_kept() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        a.record(2);
        a.switch(2);
        a.retire_older(2);

        for _ in 0..3 {
            a.check_counters(&mut |_, _, _| Err("ip failed".to_string()));
        }
        assert_eq!(a.spis(CryptoDirection::Ingress).len(), 2);
    }

    #[test]
    fn every_connection_uses_the_keys_of_the_pair_with_its_peer() {
        let crypto = RustCrypto::default();
        let to_b = DesiredConnection {
            local_ip: ip(1),
            remote_ip: ip(2),
            peer_gateway_ip: gateway(2),
        };
        let to_c = DesiredConnection {
            local_ip: ip(1),
            remote_ip: ip(3),
            peer_gateway_ip: gateway(3),
        };
        let to_d = DesiredConnection {
            local_ip: ip(1),
            remote_ip: ip(4),
            peer_gateway_ip: gateway(4),
        };
        let mut keys = NetworkKeys::new(1, vec![1u8; BASE_LENGTH], &[gateway(2)]);
        // c joins: its pair only gets the incoming keys, until it is switched
        keys.record(
            2,
            vec![2u8; BASE_LENGTH],
            &[gateway(2), gateway(3)],
            None,
            0,
        );

        let sas = desired_sas(&keys, &crypto, 5, &[to_b, to_c, to_d], gateway(1)).unwrap();
        let count = |conn: &DesiredConnection, direction: CryptoDirection| {
            sas.iter()
                .filter(|sa| sa.conn == *conn && sa.direction == direction)
                .count()
        };
        assert_eq!(count(&to_b, CryptoDirection::Ingress), 2);
        assert_eq!(count(&to_b, CryptoDirection::Egress), 1);
        assert_eq!(count(&to_c, CryptoDirection::Ingress), 1);
        assert_eq!(count(&to_c, CryptoDirection::Egress), 0);
        // d is no member, so its connection stays fail-closed
        assert_eq!(count(&to_d, CryptoDirection::Ingress), 0);
        assert_eq!(count(&to_d, CryptoDirection::Egress), 0);

        keys.pairs.get_mut(&gateway(3)).unwrap().egress = Some((1, 2));
        let sas = desired_sas(&keys, &crypto, 5, &[to_c], gateway(1)).unwrap();
        assert_eq!(sas.len(), 2);
    }
}
