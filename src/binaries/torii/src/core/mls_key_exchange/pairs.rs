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

//! The key-change of the pairs.
//!
//! The two gateways of a pair move to the keys of a new epoch together, coordinated by their
//! `Ready` and `Switched` (see `pair_messages`). Every pair moves on its own (see `step`): it only
//! waits for its own kernel and for its peer, so a slow or silent gateway only holds back its own
//! pairs. Waiting is always safe, because a pair, which doesn't move, keeps its old keys, which
//! still work on both ends.

use crate::core::mls_key_exchange::network_keys::{EpochId, NetworkKeys};
use crate::core::mls_key_exchange::pair_messages::{PairMessageKind, queue};
use crate::core::mls_key_exchange::state::MlsState;
use ainari_api_structs::network_crypto_structs::CryptoDirection;
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// Access to the keys in the kernel, which the steps of the pairs need
pub trait PairKernel {
    /// Brings the keys of the network in the kernel in line with its epochs.
    fn apply(&mut self, keys: &NetworkKeys);

    /// Checks, if the keys of an epoch are in place for all connections of a pair in one
    /// direction.
    fn installed(
        &self,
        keys: &NetworkKeys,
        peer: Ipv4Addr,
        id: EpochId,
        direction: CryptoDirection,
    ) -> bool;
}

/// Result of a step of the pairs of a network
#[derive(Debug, Default)]
pub struct Step {
    /// Messages to send, with the peers, which they concern
    pub messages: Vec<(PairMessageKind, EpochId, Vec<Ipv4Addr>)>,
    /// The state changed and has to be saved
    pub changed: bool,
}

impl Step {
    /// Adds a message for every epoch, which the peers are told about.
    fn tell(&mut self, kind: PairMessageKind, peers_by_epoch: BTreeMap<EpochId, Vec<Ipv4Addr>>) {
        for (id, peers) in peers_by_epoch {
            self.messages.push((kind, id, peers));
        }
    }
}

/// Moves every pair of a network, which has a target, as far as it can for now, after the stale
/// pairs ended:
///
/// 1. the incoming keys of the target are installed
/// 2. the peer is told with a `Ready`, that this gateway can receive with them
/// 3. after the `Ready` of the peer, the outgoing traffic switches to the target
/// 4. the peer is told with a `Switched`
/// 5. after the `Switched` of the peer, the older incoming epochs are retired, and the pair has
///    arrived, once both sides switched
///
/// A step, which fails, is tried again with the next call, `last_error` tells why. The messages
/// are not marked as sent, see `mark_sent`.
///
/// # Arguments
/// * `keys` - The epochs of the network
/// * `kernel` - Access to the keys in the kernel
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// The messages to send and if the state changed
pub fn step(keys: &mut NetworkKeys, kernel: &mut impl PairKernel, now: i64) -> Step {
    let mut result = Step::default();

    // a pair, whose peer didn't join the current group in time, ends with all of its keys
    if keys.end_stale_pairs(now) {
        kernel.apply(keys);
        result.changed = true;
    }
    install_ingress(keys, kernel, &mut result);
    ready_and_switch(keys, kernel, &mut result);
    switched_and_retire(keys, &mut result);
    result
}

/// Step 1: adds the target of every pair to its incoming epochs and installs their keys.
fn install_ingress(keys: &mut NetworkKeys, kernel: &mut impl PairKernel, result: &mut Step) {
    let mut missing = false;
    for pair in keys.pairs.values_mut() {
        if let Some(target) = pair.target
            && pair.ingress.insert(target)
        {
            missing = true;
        }
    }
    if missing {
        kernel.apply(keys);
        result.changed = true;
    }
}

/// Steps 2 and 3: tells the peer of every pair, whose incoming keys are installed, that this
/// gateway is ready, and switches the outgoing traffic of every pair, whose peer is ready.
fn ready_and_switch(keys: &mut NetworkKeys, kernel: &mut impl PairKernel, result: &mut Step) {
    let peers: Vec<Ipv4Addr> = keys.pairs.keys().copied().collect();
    let mut ready: BTreeMap<EpochId, Vec<Ipv4Addr>> = BTreeMap::new();
    let mut switching = Vec::new();
    for peer in peers {
        let Some(target) = keys.pairs[&peer].target else {
            continue;
        };
        if !kernel.installed(keys, peer, target, CryptoDirection::Ingress) {
            set_error(
                keys,
                &peer,
                format!("incoming keys of {target:?} are not installed"),
            );
            result.changed = true;
            continue;
        }
        let pair = keys.pairs.get_mut(&peer).expect("pair of the list");
        if pair.ready_sent != Some(target) {
            ready.entry(target).or_default().push(peer);
        }
        // the peer may already be ready with a newer epoch, it still receives with the target
        if pair.peer_ready >= Some(target) && pair.egress != Some(target) {
            switching.push((peer, pair.egress));
            pair.egress = Some(target);
        }
    }
    if !switching.is_empty() {
        confirm_switch(keys, kernel, switching);
        result.changed = true;
    }
    result.tell(PairMessageKind::Ready, ready);
}

/// Installs the outgoing keys of the pairs, which switch to their target, and points the
/// policies at them. A pair, whose keys couldn't be installed, stays with its previous epoch.
///
/// # Arguments
/// * `keys` - The epochs of the network, whose switching pairs have their target as egress
/// * `kernel` - Access to the keys in the kernel
/// * `switching` - The switching pairs with their previous egress
fn confirm_switch(
    keys: &mut NetworkKeys,
    kernel: &mut impl PairKernel,
    switching: Vec<(Ipv4Addr, Option<EpochId>)>,
) {
    kernel.apply(keys);
    let mut failed = false;
    for (peer, previous) in switching {
        let Some(target) = keys.pairs[&peer].target else {
            continue;
        };
        if !kernel.installed(keys, peer, target, CryptoDirection::Egress) {
            let pair = keys.pairs.get_mut(&peer).expect("pair of the list");
            pair.egress = previous;
            pair.last_error = Some(format!("outgoing keys of {target:?} are not installed"));
            failed = true;
        }
    }
    // the policies of a failed switch still point at the old keys, which are kept
    if failed {
        kernel.apply(keys);
    }
}

/// Steps 4 and 5: tells the peer of every pair, which switched, and retires the incoming epochs,
/// with which the peer doesn't send anymore. A pair has arrived, once both sides switched.
fn switched_and_retire(keys: &mut NetworkKeys, result: &mut Step) {
    let mut switched: BTreeMap<EpochId, Vec<Ipv4Addr>> = BTreeMap::new();
    for (peer, pair) in keys.pairs.iter_mut() {
        let Some(target) = pair.target else {
            continue;
        };
        if pair.egress == Some(target) && pair.switched_sent != Some(target) {
            switched.entry(target).or_default().push(*peer);
        }
        let Some(peer_switched) = pair.peer_switched else {
            continue;
        };
        let older: Vec<EpochId> = pair.ingress.range(..peer_switched).copied().collect();
        for old in older {
            pair.ingress.remove(&old);
            pair.retiring.entry(old).or_default();
            result.changed = true;
        }
        if pair.egress == Some(target) && peer_switched >= target {
            pair.target = None;
            pair.last_error = None;
            result.changed = true;
        }
    }
    result.tell(PairMessageKind::Switched, switched);
}

/// Sets the reason, why the step of a pair failed.
fn set_error(keys: &mut NetworkKeys, peer: &Ipv4Addr, error: String) {
    if let Some(pair) = keys.pairs.get_mut(peer) {
        pair.last_error = Some(error);
    }
}

/// Moves the pairs of a network on (see `step`) and queues their messages.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
/// * `kernel` - Access to the keys in the kernel
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `true` if the state changed and has to be saved
pub fn drive_pairs(state: &mut MlsState, vni: u32, kernel: &mut impl PairKernel, now: i64) -> bool {
    let Some(keys) = state.network_keys.get_mut(&vni) else {
        return false;
    };
    let step = step(keys, kernel, now);
    let mut changed = step.changed;
    for (kind, id, peers) in step.messages {
        match queue(state, vni, kind, id, &peers) {
            Ok(()) => changed = true,
            Err(e) => {
                log::error!("Failed to tell the pairs of tenant {vni} about {id:?}: {e:?}");
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::group;
    use crate::core::mls_key_exchange::keys::{DesiredConnection, remove_idle_keys};
    use crate::core::mls_key_exchange::network_keys::{NetworkKeys, STALE_PAIR_TIMEOUT};
    use crate::core::mls_key_exchange::pair_messages::{PairMessage, PairMessageKind};
    use crate::core::mls_key_exchange::state::MlsState;
    use crate::core::mls_key_exchange::test_support::{
        A, B, C, TestGateway, TestKernel, addition, addr, arrived, commit, current, deliver_to,
        drive_all, gateway, idle, in_sync, joined, new_gateway, pair, rotate, step_of,
    };

    use openmls::prelude::OpenMlsProvider;
    use openmls_rust_crypto::RustCrypto;

    #[test]
    fn the_key_change_of_a_pair_never_leaves_a_gap() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        assert!(in_sync(&a, &b));

        // a reaches the new epoch first and tells b, which doesn't know it yet
        a.record(2);
        let from_a = step_of(&mut a);
        assert!(in_sync(&a, &b));
        deliver_to(from_a, &a, &mut b);
        assert!(in_sync(&a, &b));

        // b reaches it, knows that a can receive with it and switches right away
        b.record(2);
        let from_b = step_of(&mut b);
        assert_eq!(from_b.len(), 2);
        assert!(in_sync(&a, &b));
        idle(&mut b);
        assert!(in_sync(&a, &b));

        // a switches and drops its old incoming keys
        deliver_to(from_b, &b, &mut a);
        let from_a = step_of(&mut a);
        assert!(in_sync(&a, &b));
        idle(&mut a);
        assert!(in_sync(&a, &b));
        assert!(arrived(&a, (1, 2)));

        deliver_to(from_a, &a, &mut b);
        assert!(step_of(&mut b).is_empty());
        idle(&mut b);
        assert!(in_sync(&a, &b));
        assert!(arrived(&b, (1, 2)));
        for gateway in [&a, &b] {
            assert_eq!(gateway.spis(CryptoDirection::Ingress).len(), 1);
        }
        // nothing is left to do
        assert!(step_of(&mut a).is_empty());
    }

    #[test]
    fn a_new_target_replaces_the_one_a_pair_is_moving_to() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        a.record(2);
        b.record(2);
        let from_a = step_of(&mut a);
        deliver_to(from_a, &a, &mut b);
        // b switched to epoch 2, before the group moves on to epoch 3
        let from_b = step_of(&mut b);
        a.record(3);
        b.record(3);
        assert!(in_sync(&a, &b));
        deliver_to(from_b, &b, &mut a);

        for _ in 0..3 {
            let from_a = step_of(&mut a);
            let from_b = step_of(&mut b);
            assert!(in_sync(&a, &b));
            deliver_to(from_a, &a, &mut b);
            deliver_to(from_b, &b, &mut a);
            idle(&mut a);
            idle(&mut b);
            assert!(in_sync(&a, &b));
        }
        assert!(arrived(&a, (1, 3)));
        assert!(arrived(&b, (1, 3)));
        assert_eq!(
            a.keys.bases.keys().copied().collect::<Vec<_>>(),
            vec![(1, 3)]
        );
    }

    #[test]
    fn a_replaced_group_moves_the_pair_without_a_gap() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);

        // izakaya lost its database: a starts a new group, b joins it later. b can't read the
        // ready of the new group yet.
        a.replace(0);
        let from_a = step_of(&mut a);
        deliver_to(from_a, &a, &mut b);
        assert!(in_sync(&a, &b));
        a.record_new_group(1);
        b.replace(1);
        assert!(in_sync(&a, &b));

        for _ in 0..3 {
            let from_a = step_of(&mut a);
            let from_b = step_of(&mut b);
            assert!(in_sync(&a, &b));
            deliver_to(from_a, &a, &mut b);
            deliver_to(from_b, &b, &mut a);
            idle(&mut a);
            idle(&mut b);
            assert!(in_sync(&a, &b));
        }
        assert!(arrived(&a, (2, 1)));
        assert!(arrived(&b, (2, 1)));
        assert_eq!(
            a.keys.bases.keys().copied().collect::<Vec<_>>(),
            vec![(2, 1)]
        );
    }

    #[test]
    fn without_a_switched_the_old_incoming_keys_are_never_removed() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        a.record(2);
        b.record(2);
        let from_a = step_of(&mut a);
        deliver_to(from_a, &a, &mut b);
        // the switched of b gets lost, only its ready arrives
        let from_b: Vec<PairMessage> = step_of(&mut b)
            .into_iter()
            .filter(|message| message.kind == PairMessageKind::Ready)
            .collect();
        deliver_to(from_b, &b, &mut a);

        for _ in 0..5 {
            step_of(&mut a);
            idle(&mut a);
        }
        assert!(in_sync(&a, &b));
        assert_eq!(a.spis(CryptoDirection::Ingress).len(), 2);
        assert!(!arrived(&a, (1, 2)));
    }

    #[test]
    fn a_failed_switch_keeps_the_old_outgoing_keys() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        a.record(2);
        b.record(2);
        let from_b = step_of(&mut b);
        deliver_to(from_b, &b, &mut a);

        let step = step(
            &mut a.keys,
            &mut TestKernel {
                refuse_egress: true,
            },
            0,
        );
        let pair = &a.keys.pairs[&gateway(2)];
        assert_eq!(pair.egress, Some((1, 1)));
        assert!(pair.last_error.is_some());
        assert!(
            step.messages
                .iter()
                .all(|(kind, _, _)| *kind == PairMessageKind::Ready)
        );

        // the next step succeeds
        step_of(&mut a);
        assert_eq!(a.keys.pairs[&gateway(2)].egress, Some((1, 2)));
    }

    #[test]
    fn a_pair_whose_peer_does_not_join_the_new_group_ends_after_a_while() {
        let (b, c) = (gateway(2), gateway(3));
        let mut keys = NetworkKeys::new(1, vec![1u8; 32], &[b, c]);

        // izakaya lost its database, the new group only has b so far
        keys.replace(0, vec![2u8; 32], &[b], 1000);
        let pair = &keys.pairs[&c];
        assert_eq!(pair.stale_since, Some(1000));
        assert_eq!(pair.egress, Some((1, 1)));
        assert_ne!(pair.target, Some((2, 0)));

        // c keeps its keys, also with the next commit of the new group
        keys.record(1, vec![3u8; 32], &[b], None, 1050);
        assert_eq!(keys.pairs[&c].stale_since, Some(1000));
        step(
            &mut keys,
            &mut TestKernel::default(),
            1000 + STALE_PAIR_TIMEOUT,
        );
        assert!(keys.pairs.contains_key(&c));

        // until it is stale for too long
        let result = step(
            &mut keys,
            &mut TestKernel::default(),
            1001 + STALE_PAIR_TIMEOUT,
        );
        assert!(result.changed);
        assert!(!keys.pairs.contains_key(&c));
        assert!(keys.pairs.contains_key(&b));
    }

    #[test]
    fn a_stale_pair_moves_on_when_its_peer_joins() {
        let (b, c) = (gateway(2), gateway(3));
        let mut keys = NetworkKeys::new(1, vec![1u8; 32], &[b, c]);
        keys.replace(0, vec![2u8; 32], &[b], 1000);

        keys.record(1, vec![3u8; 32], &[b, c], None, 1010);
        let pair = &keys.pairs[&c];
        assert_eq!(pair.stale_since, None);
        assert_eq!(pair.target, Some((2, 1)));
        // until it switched, the pair keeps the keys of the old group
        assert_eq!(pair.egress, Some((1, 1)));

        // b leaves the new group, which ends its pair right away
        keys.record(2, vec![4u8; 32], &[c], None, 1020);
        assert!(!keys.pairs.contains_key(&b));
    }

    #[test]
    fn a_gateway_which_never_answers_only_blocks_its_own_pairs() {
        let mut gateways = joined(&[A, B, C]);
        let old = current(&gateways[0]);
        rotate(&mut gateways, &[2]);
        let new = current(&gateways[0]);
        for _ in 0..3 {
            drive_all(&mut gateways, &[2]);
        }

        for (gateway, peer) in [(0, B), (1, A)] {
            let pair = &gateways[gateway].network_keys[&5].pairs[&addr(peer)];
            assert_eq!(pair.egress, Some(new));
            assert_eq!(pair.target, None);
        }
        // the pairs with c still send with the old keys, which c knows
        for gateway in [0, 1] {
            let pair = &gateways[gateway].network_keys[&5].pairs[&addr(C)];
            assert_eq!(pair.egress, Some(old));
            assert_eq!(pair.target, Some(new));
            assert!(pair.ingress.contains(&old));
        }
    }

    #[test]
    fn a_restart_between_ready_and_switched_sends_nothing_twice() {
        let mut gateways = joined(&[A, B]);
        rotate(&mut gateways, &[]);
        drive_pairs(&mut gateways[0], 5, &mut TestKernel::default(), 0);
        assert_eq!(gateways[0].pending_out.len(), 1);

        // a restarts with its ready still in its queue
        gateways[0].save().unwrap();
        let values = gateways[0]
            .provider
            .storage()
            .values
            .read()
            .unwrap()
            .clone();
        let mut restored = MlsState::new_in_memory();
        restored.restore(values).unwrap();
        gateways[0] = restored;
        drive_pairs(&mut gateways[0], 5, &mut TestKernel::default(), 0);
        assert_eq!(gateways[0].pending_out.len(), 1);

        // the exchange goes on with the queued ready
        let mut sent = 0;
        for _ in 0..3 {
            sent += drive_all(&mut gateways, &[])[0];
        }
        // ready and switched, each once
        assert_eq!(sent, 2);
        let new = current(&gateways[0]);
        let pair = &gateways[0].network_keys[&5].pairs[&addr(B)];
        assert_eq!(pair.egress, Some(new));
        assert_eq!(pair.target, None);
    }

    #[test]
    fn a_join_does_not_change_the_keys_between_the_others() {
        let mut gateways = joined(&[A, B]);
        for _ in 0..2 {
            drive_all(&mut gateways, &[]);
        }
        let before = (pair(&gateways[0], B), pair(&gateways[1], A));
        assert_eq!(before.0.target, None);
        let old = current(&gateways[0]);

        gateways.push(new_gateway(C));
        let added = addition(&gateways[0], &gateways[2], 5);
        commit(&mut gateways, vec![added], &[]);
        let new = current(&gateways[0]);
        for _ in 0..3 {
            drive_all(&mut gateways, &[]);
        }

        // the pair of a and b didn't move, so c can't derive its keys
        assert_eq!((pair(&gateways[0], B), pair(&gateways[1], A)), before);
        assert!(!gateways[2].network_keys[&5].bases.contains_key(&old));
        // the pairs with c use the epoch of the join
        for (gateway, peer) in [(0, C), (1, C), (2, A), (2, B)] {
            let pair = pair(&gateways[gateway], peer);
            assert_eq!(pair.egress, Some(new));
            assert_eq!(pair.target, None);
        }
    }

    #[test]
    fn a_re_add_only_moves_the_pairs_of_the_re_added_gateway() {
        let mut gateways = joined(&[A, B, C]);
        let before = pair(&gateways[0], B);

        // c lost its group and is added again
        group::delete_group(&mut gateways[2], 5).unwrap();
        gateways[2].network_keys.clear();
        let added = addition(&gateways[0], &gateways[2], 5);
        commit(&mut gateways, vec![added], &[]);

        assert_eq!(pair(&gateways[0], B), before);
        assert_eq!(pair(&gateways[0], C).target, Some(current(&gateways[0])));
    }

    #[test]
    fn a_removal_moves_every_remaining_pair_away_from_the_keys_of_the_removed_gateway() {
        let mut gateways = joined(&[A, B, C]);
        let known_to_c = current(&gateways[2]);

        commit(&mut gateways, Vec::new(), &[C.to_string()]);
        for gateway in [0, 1] {
            assert!(
                !gateways[gateway].network_keys[&5]
                    .pairs
                    .contains_key(&addr(C))
            );
        }
        let new = current(&gateways[0]);
        assert!(new > known_to_c);
        for _ in 0..3 {
            drive_all(&mut gateways[..2], &[]);
        }

        // the old incoming keys of the pair of a and b are idle and go
        let conn = DesiredConnection {
            local_ip: Ipv4Addr::new(192, 168, 0, 1),
            remote_ip: Ipv4Addr::new(192, 168, 0, 2),
            peer_gateway_ip: addr(B),
        };
        let keys = gateways[0].network_keys.get_mut(&5).unwrap();
        for _ in 0..2 {
            remove_idle_keys(
                keys,
                &RustCrypto::default(),
                5,
                &[conn],
                addr(A),
                &mut |_, _, _| Ok(Some(0)),
            );
        }
        let pair = &keys.pairs[&addr(B)];
        assert_eq!(pair.egress, Some(new));
        assert_eq!(pair.ingress, std::collections::BTreeSet::from([new]));
        assert!(pair.retiring.is_empty());
        assert!(keys.bases.keys().all(|id| *id > known_to_c));
    }

    #[test]
    fn a_self_update_moves_every_pair() {
        let mut gateways = joined(&[A, B, C]);
        commit(&mut gateways, Vec::new(), &[]);
        let new = current(&gateways[0]);
        for peer in [B, C] {
            assert_eq!(pair(&gateways[0], peer).target, Some(new));
        }
        for _ in 0..3 {
            drive_all(&mut gateways, &[]);
        }
        for (gateway, peer) in [(0, B), (0, C), (1, A), (1, C), (2, A), (2, B)] {
            assert_eq!(pair(&gateways[gateway], peer).egress, Some(new));
        }
    }
}
