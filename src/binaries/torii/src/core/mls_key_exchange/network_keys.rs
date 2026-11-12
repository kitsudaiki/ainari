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

//! Key state of the networks: the epochs and the pairs of every network.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;

/// Seconds, after which a pair ends, whose peer didn't join the group, which replaced the older
/// group of the network, so its connections fall back to their fail-closed block policies
pub const STALE_PAIR_TIMEOUT: i64 = 120;

/// Id of an epoch of a network on this gateway: the generation of the group and the epoch within
/// the group. The generation counts the groups of the network, which the gateway created or
/// joined, so the epochs of a newer group are always newer than the ones of an older group.
pub type EpochId = (u64, u64);

/// The keys between this gateway and one peer gateway in a network. All connections between the
/// VMs of the two gateways change their keys together.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PairKeys {
    /// Epoch of the outgoing keys towards the peer, `None` before the first switch
    pub egress: Option<EpochId>,
    /// Epochs of the incoming keys from the peer
    pub ingress: BTreeSet<EpochId>,
    /// Epoch, which the pair is moving to
    pub target: Option<EpochId>,
    /// Old epochs, whose incoming keys wait for their removal, with the packet counters of their
    /// keys by SPI at the last check. The counters of an epoch are empty, until they are read the
    /// first time.
    #[serde(default, with = "epoch_counters")]
    pub retiring: BTreeMap<EpochId, BTreeMap<u32, u64>>,
    /// Latest epoch, with which the peer told it can receive
    pub peer_ready: Option<EpochId>,
    /// Latest epoch, with which the peer told it sends
    pub peer_switched: Option<EpochId>,
    /// Latest epoch, with which this gateway told the peer it can receive
    pub ready_sent: Option<EpochId>,
    /// Latest epoch, with which this gateway told the peer it sends
    pub switched_sent: Option<EpochId>,
    /// Why the last step of the pair failed, if it did
    pub last_error: Option<String>,
    /// Unix-time in seconds, since which the peer is no member of the current group, because it
    /// didn't join the group yet, which replaced an older one (see `STALE_PAIR_TIMEOUT`)
    pub stale_since: Option<i64>,
}

impl PairKeys {
    /// Checks, if the pair already has keys of a generation.
    fn knows_generation(&self, generation: u64) -> bool {
        self.ingress
            .iter()
            .chain(self.retiring.keys())
            .copied()
            .chain(self.egress)
            .chain(self.target)
            .any(|id| id.0 == generation)
    }
}

/// The epochs of a network, whose IPsec keys are installed on the gateway.
///
/// openmls can only export secrets of the current epoch of a group, but a key-rotation needs
/// the keys of two epochs at the same time: the incoming keys of the new epoch are installed
/// first, while the outgoing traffic still uses the old epoch, until the peer can receive with
/// the new one. So a base secret is exported from every epoch, as soon as the gateway enters it,
/// and the keys of the connections are derived from that base secret.
///
/// Every new epoch becomes the target of the pairs, which then move to it on their own, only
/// coordinated with their peer (see `pairs::step`).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct NetworkKeys {
    /// Generation of the current group of the network
    pub generation: u64,
    /// Base secrets by epoch, kept as long as a pair uses the epoch
    #[serde(with = "epoch_map")]
    pub bases: BTreeMap<EpochId, Vec<u8>>,
    /// The pairs of the network, by the identity (underlay-address) of the peer
    pub pairs: BTreeMap<Ipv4Addr, PairKeys>,
}

/// Stores the counters of the retiring epochs as a list, because JSON only knows strings as keys.
mod epoch_counters {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::EpochId;

    type Counters = BTreeMap<EpochId, BTreeMap<u32, u64>>;

    pub fn serialize<S: Serializer>(map: &Counters, serializer: S) -> Result<S::Ok, S::Error> {
        map.iter()
            .map(|(id, counters)| (id, counters.iter().collect::<Vec<_>>()))
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Counters, D::Error> {
        let entries = Vec::<(EpochId, Vec<(u32, u64)>)>::deserialize(deserializer)?;
        Ok(entries
            .into_iter()
            .map(|(id, counters)| (id, counters.into_iter().collect()))
            .collect())
    }
}

/// Stores a map with epoch ids as keys as a list, because JSON only knows strings as keys.
mod epoch_map {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::EpochId;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<EpochId, Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<EpochId, Vec<u8>>, D::Error> {
        let entries = Vec::<(EpochId, Vec<u8>)>::deserialize(deserializer)?;
        Ok(entries.into_iter().collect())
    }
}

impl NetworkKeys {
    /// Starts the keys of a network with the first epoch of a group, which every pair uses right
    /// away. The epoch is still the target of the pairs, so they tell their peers with `Ready` and
    /// `Switched`: the other members recorded the epoch with the commit, which added this gateway,
    /// and wait for the `Ready`, before they send with it.
    ///
    /// # Arguments
    /// * `epoch` - The first epoch
    /// * `base` - Base secret of the epoch
    /// * `peers` - The other members of the group
    ///
    /// # Returns
    /// The keys with the epoch as the only and active one
    pub fn new(epoch: u64, base: Vec<u8>, peers: &[Ipv4Addr]) -> Self {
        let id = (1, epoch);
        let pair = PairKeys {
            egress: Some(id),
            ingress: BTreeSet::from([id]),
            target: Some(id),
            ..Default::default()
        };
        NetworkKeys {
            generation: 1,
            bases: BTreeMap::from([(id, base)]),
            pairs: peers.iter().map(|peer| (*peer, pair.clone())).collect(),
        }
    }

    /// Records a new epoch of the current group: it becomes the target of the moving pairs and
    /// their incoming keys are installed, while the outgoing traffic stays with the old epochs.
    /// The other pairs keep their epochs.
    ///
    /// The pair of a peer, which left the current group, ends with all of its keys. The pair of a
    /// peer, which only didn't join the current group yet, keeps its keys of the older group, but
    /// becomes stale.
    ///
    /// # Arguments
    /// * `epoch` - The new epoch
    /// * `base` - Base secret of the epoch
    /// * `peers` - The other members of the group. A peer without a pair gets one, which has no
    ///   outgoing keys until the switch.
    /// * `moving` - The peers, whose pairs move to the new epoch, or `None` for all members. A new
    ///   pair always moves.
    /// * `now` - Current unix-time in seconds
    pub fn record(
        &mut self,
        epoch: u64,
        base: Vec<u8>,
        peers: &[Ipv4Addr],
        moving: Option<&[Ipv4Addr]>,
        now: i64,
    ) {
        let generation = self.generation;
        let id = (generation, epoch);
        self.bases.insert(id, base);
        self.pairs.retain(|peer, pair| {
            if peers.contains(peer) {
                return true;
            }
            if pair.knows_generation(generation) {
                log::warn!(
                    "{peer} left the group of generation {generation}, its keys are removed"
                );
                return false;
            }
            pair.stale_since.get_or_insert(now);
            true
        });
        for peer in peers {
            let is_new = !self.pairs.contains_key(peer);
            if !is_new && moving.is_some_and(|moving| !moving.contains(peer)) {
                continue;
            }
            let pair = self.pairs.entry(*peer).or_default();
            pair.ingress.insert(id);
            pair.target = Some(id);
            pair.stale_since = None;
        }
        self.drop_unused_bases();
    }

    /// Records the first epoch of a new group of the network, which replaces the current group,
    /// for example after izakaya lost its database. It is the next generation, so the pairs keep
    /// the keys of the older group, until they switched to the new one.
    ///
    /// # Arguments
    /// * `epoch` - The first epoch of the new group
    /// * `base` - Base secret of the epoch
    /// * `peers` - The other members of the new group
    /// * `now` - Current unix-time in seconds
    pub fn replace(&mut self, epoch: u64, base: Vec<u8>, peers: &[Ipv4Addr], now: i64) {
        self.generation += 1;
        self.record(epoch, base, peers, None, now);
    }

    /// Ends the pairs, which are stale for longer than `STALE_PAIR_TIMEOUT`.
    ///
    /// # Arguments
    /// * `now` - Current unix-time in seconds
    ///
    /// # Returns
    /// `true` if a pair ended
    pub fn end_stale_pairs(&mut self, now: i64) -> bool {
        let before = self.pairs.len();
        self.pairs.retain(|peer, pair| match pair.stale_since {
            Some(since) if now - since > STALE_PAIR_TIMEOUT => {
                log::warn!("{peer} didn't join the new group in time, its keys are removed");
                false
            }
            _ => true,
        });
        if self.pairs.len() == before {
            return false;
        }
        self.drop_unused_bases();
        true
    }

    /// Drops the base secrets, which no pair uses anymore.
    pub fn drop_unused_bases(&mut self) {
        let used: BTreeSet<EpochId> = self
            .pairs
            .values()
            .flat_map(|pair| {
                pair.ingress
                    .iter()
                    .chain(pair.retiring.keys())
                    .copied()
                    .chain(pair.egress)
                    .chain(pair.target)
            })
            .collect();
        self.bases.retain(|id, _| used.contains(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The peer of the pairs within the tests
    const PEER: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 9);

    /// Builds the epochs of a network with one pair, which sends with `active_epoch`.
    fn network_keys(active_epoch: u64, epochs: &[u64]) -> NetworkKeys {
        NetworkKeys {
            generation: 1,
            bases: epochs
                .iter()
                .map(|epoch| ((1, *epoch), vec![0u8; 32]))
                .collect(),
            pairs: BTreeMap::from([(
                PEER,
                PairKeys {
                    egress: Some((1, active_epoch)),
                    ingress: epochs.iter().map(|epoch| (1, *epoch)).collect(),
                    target: epochs.last().map(|epoch| (1, *epoch)),
                    ..Default::default()
                },
            )]),
        }
    }

    #[test]
    fn the_pair_of_a_removed_peer_ends_with_the_commit() {
        let other = Ipv4Addr::new(10, 0, 0, 8);
        let mut keys = network_keys(1, &[1]);
        keys.record(2, vec![2u8; 32], &[PEER, other], None, 0);
        assert_eq!(keys.pairs.len(), 2);

        // other is removed
        keys.record(3, vec![3u8; 32], &[PEER], None, 0);
        assert_eq!(keys.pairs.keys().copied().collect::<Vec<_>>(), vec![PEER]);
        assert_eq!(keys.pairs[&PEER].target, Some((1, 3)));
    }

    #[test]
    fn a_replaced_group_keeps_the_old_keys() {
        let mut keys = network_keys(2, &[2]);
        let other = Ipv4Addr::new(10, 0, 0, 8);

        // the new group has a new member, the old peer joins it as well
        keys.replace(0, vec![1u8; 32], &[PEER, other], 0);
        assert_eq!(keys.generation, 2);
        assert_eq!(keys.pairs[&PEER].egress, Some((1, 2)));
        assert_eq!(keys.pairs[&PEER].target, Some((2, 0)));
        assert_eq!(keys.pairs[&other].egress, None);

        // the epochs of the new group are newer than the ones of the old group
        assert!(keys.pairs[&PEER].target > keys.pairs[&PEER].egress);
        assert_eq!(
            keys.bases.keys().copied().collect::<Vec<_>>(),
            vec![(1, 2), (2, 0)]
        );
    }

    #[test]
    fn the_key_state_survives_a_restore() {
        let keys = network_keys(1, &[1, 2]);
        let json = serde_json::to_vec(&keys).unwrap();
        let restored: NetworkKeys = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored.generation, keys.generation);
        assert_eq!(restored.bases, keys.bases);
        assert_eq!(restored.pairs, keys.pairs);
    }
}
