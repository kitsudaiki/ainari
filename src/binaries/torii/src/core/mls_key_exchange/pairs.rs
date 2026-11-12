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

//! Messages between the two gateways of a pair.
//!
//! The gateways of a pair tell each other, when they can receive with the keys of an epoch
//! (`Ready`) and when they send with them (`Switched`). The messages are MLS application messages:
//! encrypted within the group and signed by the leaf of the sender, so izakaya can neither read
//! nor forge them, and a member can only speak for its own pairs.
//!
//! Every pair moves on its own (see `step`): it only waits for its own kernel and for its peer,
//! so a slow or silent gateway only holds back its own pairs. Waiting is always safe, because a
//! pair, which doesn't move, keeps its old keys, which still work on both ends.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use openmls::prelude::GroupId;
use serde::{Deserialize, Serialize};

use crate::core::mls_key_exchange::group::{self, MAX_PAST_EPOCHS, OutgoingMessage, Processed};
use crate::core::mls_key_exchange::state::{EpochId, MlsState, NetworkKeys, PairKeys, vni_of};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::{MlsMessageResp, MlsMessageType, VerifyingKey};
use ainari_api_structs::network_crypto_structs::{
    CryptoDirection, MlsEpochIdResp, MlsNetworkStatusResp, MlsPairStatusResp, MlsPairWaitingFor,
    MlsStatusResp,
};

/// What a gateway tells the peers of its pairs about an epoch
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PairMessageKind {
    /// The gateway can receive with the keys of the epoch
    Ready,
    /// The gateway sends with the keys of the epoch
    Switched,
}

/// Plaintext of an application message between the gateways of pairs
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PairMessage {
    pub kind: PairMessageKind,
    pub generation: u64,
    pub epoch: u64,
    /// Identities of the peers, which the message concerns
    pub peers: Vec<String>,
}

/// Queues a message about an epoch for the peers of pairs of a network, which is sent with the
/// next call of `flush`.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
/// * `kind` - What the message tells
/// * `id` - The epoch
/// * `peers` - The peers of the pairs
///
/// # Returns
/// `Ok(())` once the message is queued, or the error of the encryption
pub fn queue(
    state: &mut MlsState,
    vni: u32,
    kind: PairMessageKind,
    id: EpochId,
    peers: &[Ipv4Addr],
) -> Result<(), ErrorResponse> {
    if peers.is_empty() {
        return Ok(());
    }
    let recipients: Vec<String> = peers.iter().map(|peer| peer.to_string()).collect();
    let message = PairMessage {
        kind,
        generation: id.0,
        epoch: id.1,
        peers: recipients.clone(),
    };
    let content = serde_json::to_vec(&message)
        .map_err(|e| group::internal_error("encode a message of a pair", e))?;
    let outgoing = group::create_application_message(state, vni, &content, recipients)?;
    state.pending_out.push(outgoing);

    if let Some(keys) = state.network_keys.get_mut(&vni) {
        mark_sent(keys, kind, id, peers);
    }
    Ok(())
}

/// Remembers, that the peers of pairs were told about an epoch, so it is never told twice.
///
/// # Arguments
/// * `keys` - The epochs of the network
/// * `kind` - What the peers were told
/// * `id` - The epoch
/// * `peers` - The peers of the pairs
pub fn mark_sent(keys: &mut NetworkKeys, kind: PairMessageKind, id: EpochId, peers: &[Ipv4Addr]) {
    for peer in peers {
        if let Some(pair) = keys.pairs.get_mut(peer) {
            match kind {
                PairMessageKind::Ready => pair.ready_sent = Some(id),
                PairMessageKind::Switched => pair.switched_sent = Some(id),
            }
        }
    }
}

/// Takes a message from the peer of a pair.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
/// * `sender` - Identity of the sender, as authenticated by the group
/// * `message` - The message
///
/// # Returns
/// `true` if the pair changed
pub fn receive(state: &mut MlsState, vni: u32, sender: &str, message: &PairMessage) -> bool {
    let Some(own) = state.client_id().map(str::to_string) else {
        return false;
    };
    let Ok(sender) = sender.parse::<Ipv4Addr>() else {
        return false;
    };
    match state.network_keys.get_mut(&vni) {
        Some(keys) => take(keys, &own, sender, message),
        None => false,
    }
}

/// Takes a message from the peer of a pair into the keys of a network.
///
/// The message only counts for the pair with its sender, and only if it is meant for this
/// gateway and belongs to the current generation of the network. It never moves a pair back, so
/// a replayed or delayed message changes nothing.
///
/// # Arguments
/// * `keys` - The epochs of the network
/// * `own` - Identity of this gateway
/// * `sender` - The sender, as authenticated by the group
/// * `message` - The message
///
/// # Returns
/// `true` if the pair changed
pub fn take(keys: &mut NetworkKeys, own: &str, sender: Ipv4Addr, message: &PairMessage) -> bool {
    if !message.peers.iter().any(|peer| peer == own) {
        log::debug!(
            "Ignore {:?} of {sender}, which is not meant for this gateway",
            message.kind
        );
        return false;
    }
    if message.generation != keys.generation {
        log::debug!(
            "Ignore {:?} of {sender} for generation {}, which is not the current one",
            message.kind,
            message.generation
        );
        return false;
    }
    let Some(pair) = keys.pairs.get_mut(&sender) else {
        return false;
    };

    let id = Some((message.generation, message.epoch));
    let latest = match message.kind {
        PairMessageKind::Ready => &mut pair.peer_ready,
        PairMessageKind::Switched => &mut pair.peer_switched,
    };
    if id <= *latest {
        return false;
    }
    *latest = id;
    log::debug!(
        "{sender} is {:?} with epoch {} of generation {}",
        message.kind,
        message.epoch,
        message.generation
    );
    true
}

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
    let peers: Vec<Ipv4Addr> = keys.pairs.keys().copied().collect();
    let target_of = |keys: &NetworkKeys, peer: &Ipv4Addr| keys.pairs[peer].target;

    // 1. incoming keys
    let mut missing = false;
    for peer in &peers {
        let pair = keys.pairs.get_mut(peer).expect("pair of the list");
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

    // 2. ready and 3. outgoing keys
    let mut ready: BTreeMap<EpochId, Vec<Ipv4Addr>> = BTreeMap::new();
    let mut switching = Vec::new();
    for peer in &peers {
        let Some(target) = target_of(keys, peer) else {
            continue;
        };
        if !kernel.installed(keys, *peer, target, CryptoDirection::Ingress) {
            set_error(
                keys,
                peer,
                format!("incoming keys of {target:?} are not installed"),
            );
            result.changed = true;
            continue;
        }
        let pair = keys.pairs.get_mut(peer).expect("pair of the list");
        if pair.ready_sent != Some(target) {
            ready.entry(target).or_default().push(*peer);
        }
        // the peer may already be ready with a newer epoch, it still receives with the target
        if pair.peer_ready >= Some(target) && pair.egress != Some(target) {
            switching.push((*peer, pair.egress));
            pair.egress = Some(target);
        }
    }
    if !switching.is_empty() {
        kernel.apply(keys);
        result.changed = true;
        let mut failed = false;
        for (peer, previous) in switching {
            let Some(target) = target_of(keys, &peer) else {
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

    // 4. switched and 5. older incoming keys
    let mut switched: BTreeMap<EpochId, Vec<Ipv4Addr>> = BTreeMap::new();
    for peer in &peers {
        let pair = keys.pairs.get_mut(peer).expect("pair of the list");
        let Some(target) = pair.target else {
            continue;
        };
        if pair.egress == Some(target) && pair.switched_sent != Some(target) {
            switched.entry(target).or_default().push(*peer);
        }
        if let Some(peer_switched) = pair.peer_switched {
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
    }

    for (id, peers) in ready {
        result.messages.push((PairMessageKind::Ready, id, peers));
    }
    for (id, peers) in switched {
        result.messages.push((PairMessageKind::Switched, id, peers));
    }
    result
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

/// Processes an application message of another gateway. A message of an epoch, which the group
/// didn't reach yet, waits in `pending_in` for the commit into that epoch.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `message` - The message, as fetched from izakaya
/// * `grant_key` - Public key of hanami
///
/// # Returns
/// `true` if the state changed, or the reason, why the message can't be processed
pub fn handle_message(
    state: &mut MlsState,
    message: &MlsMessageResp,
    grant_key: &VerifyingKey,
) -> Result<bool, String> {
    let processed = group::process_message(
        state,
        MlsMessageType::Application,
        &message.payload,
        &message.grants,
        grant_key,
    )?;
    match processed {
        Processed::Application {
            vni,
            sender,
            content,
        } => {
            let pair_message: PairMessage = serde_json::from_slice(&content)
                .map_err(|e| format!("broken message of a pair: {e}"))?;
            Ok(receive(state, vni, &sender, &pair_message))
        }
        Processed::Pending => {
            state.pending_in.push(message.clone());
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Processes the waiting messages again, after a group moved into a new epoch. A message, whose
/// epoch is too old to be decrypted anymore, or whose group is gone, is dropped.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `grant_key` - Public key of hanami
///
/// # Returns
/// `true` if the state changed
pub fn retry_pending(state: &mut MlsState, grant_key: &VerifyingKey) -> bool {
    let waiting = std::mem::take(&mut state.pending_in);
    let changed = !waiting.is_empty();
    for message in waiting {
        let epoch = vni_of(&GroupId::from_slice(message.group_id.as_bytes()))
            .and_then(|vni| state.groups.get(&vni))
            .map(|group| group.epoch().as_u64());
        let Some(epoch) = epoch else {
            log::debug!("Drop message '{}' of a group, which is gone", message.uuid);
            continue;
        };
        if message.epoch + (MAX_PAST_EPOCHS as u64) < epoch {
            log::warn!(
                "Drop message '{}' of epoch {} of group '{}', which is too old",
                message.uuid,
                message.epoch,
                message.group_id
            );
            continue;
        }
        if let Err(e) = handle_message(state, &message, grant_key) {
            log::warn!(
                "Drop message '{}' of group '{}' from '{}': {e}",
                message.uuid,
                message.group_id,
                message.sender
            );
        }
    }
    changed
}

/// Hands the queued messages to izakaya in the order of sending. A message stays queued, until
/// izakaya accepted it, so it is sent again with the next call.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `send` - Hands a message to izakaya
///
/// # Returns
/// `true` if a message was sent
pub async fn flush<F, Fut>(state: &mut MlsState, mut send: F) -> bool
where
    F: FnMut(OutgoingMessage) -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    let mut changed = false;
    while let Some(message) = state.pending_out.first().cloned() {
        if let Err(e) = send(message).await {
            log::warn!("Failed to send a message of a pair, retry it later: {e}");
            break;
        }
        state.pending_out.remove(0);
        changed = true;
    }
    changed
}

/// Tells, which step of `step` a pair waits for.
///
/// # Arguments
/// * `pair` - The pair
///
/// # Returns
/// The step
pub fn waiting_for(pair: &PairKeys) -> MlsPairWaitingFor {
    match pair.target {
        // the ready is queued in the same step, which installed the incoming keys
        Some(target) if pair.ready_sent != Some(target) => MlsPairWaitingFor::Ingress,
        Some(target) if pair.egress != Some(target) => match pair.peer_ready >= Some(target) {
            true => MlsPairWaitingFor::Egress,
            false => MlsPairWaitingFor::PeerReady,
        },
        Some(_) => MlsPairWaitingFor::PeerSwitched,
        None if !pair.retiring.is_empty() => MlsPairWaitingFor::OldIngressInUse,
        None => MlsPairWaitingFor::Nothing,
    }
}

/// Describes the state of the key-exchange of the gateway.
///
/// # Arguments
/// * `state` - The MLS-client
///
/// # Returns
/// The pairs of all networks with their epochs and the step, which they wait for
pub fn status(state: &MlsState) -> MlsStatusResp {
    let epoch_id = |id: &EpochId| MlsEpochIdResp {
        generation: id.0,
        epoch: id.1,
    };
    let networks = state
        .network_keys
        .iter()
        .map(|(vni, keys)| MlsNetworkStatusResp {
            vni: *vni,
            generation: keys.generation,
            epoch: state.groups.get(vni).map(|group| group.epoch().as_u64()),
            pairs: keys
                .pairs
                .iter()
                .map(|(peer, pair)| MlsPairStatusResp {
                    peer: peer.to_string(),
                    egress: pair.egress.as_ref().map(epoch_id),
                    target: pair.target.as_ref().map(epoch_id),
                    ingress: pair.ingress.iter().map(epoch_id).collect(),
                    retiring: pair.retiring.keys().map(epoch_id).collect(),
                    waiting_for: waiting_for(pair),
                    stale_since: pair.stale_since,
                    last_error: pair.last_error.clone(),
                })
                .collect(),
        })
        .collect();
    MlsStatusResp {
        client_id: state.client_id().map(str::to_string),
        networks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openmls::prelude::OpenMlsProvider;
    use uuid::Uuid;

    use crate::core::mls_key_exchange::group::tests::{
        addition, deliver, grant_key, invite, new_gateway,
    };
    use crate::core::mls_key_exchange::group::{Addition, pending_change, stage_change};
    use crate::core::mls_key_exchange::group::{
        CommitChange, create_group, merge_pending, stage_update,
    };
    use crate::core::mls_key_exchange::keys::tests::{TestGateway, gateway, in_sync};
    use crate::core::mls_key_exchange::keys::{DesiredConnection, remove_idle_keys};
    use crate::core::mls_key_exchange::state::PairKeys;
    use crate::core::mls_key_exchange::state::STALE_PAIR_TIMEOUT;
    use openmls_rust_crypto::RustCrypto;

    const A: &str = "10.0.0.1";
    const B: &str = "10.0.0.2";
    const C: &str = "10.0.0.3";

    fn addr(id: &str) -> Ipv4Addr {
        id.parse().unwrap()
    }

    /// Builds a group of the gateways, whose keys are recorded and in use. The pairs still have
    /// to tell each other, as after a join.
    fn joined(ids: &[&str]) -> Vec<MlsState> {
        let mut gateways: Vec<MlsState> = ids.iter().map(|id| new_gateway(id)).collect();
        create_group(&mut gateways[0], 5).unwrap();
        for i in 1..gateways.len() {
            let (first, rest) = gateways.split_at_mut(i);
            let (invited, _) = rest.split_first_mut().unwrap();
            let (inviter, others) = first.split_first_mut().unwrap();
            let mut others: Vec<&mut MlsState> = others.iter_mut().collect();
            invite(inviter, invited, &mut others, 5);
        }
        for gateway in &mut gateways {
            gateway.reset_epochs(5).unwrap();
        }
        gateways
    }

    /// Turns a queued message into the form, in which izakaya hands it to a recipient
    fn as_received(message: &OutgoingMessage, sender: &str) -> MlsMessageResp {
        MlsMessageResp {
            uuid: Uuid::new_v4(),
            group_id: message.group_id.clone(),
            epoch: message.epoch,
            message_type: message.message_type,
            sender: sender.to_string(),
            recipient: message.recipients[0].clone(),
            payload: message.payload.clone(),
            grants: message.grants.clone(),
            created_at: Utc::now(),
        }
    }

    fn current(state: &MlsState) -> EpochId {
        let keys = &state.network_keys[&5];
        (keys.generation, state.groups[&5].epoch().as_u64())
    }

    #[test]
    fn a_ready_is_taken_by_the_pair_of_its_sender() {
        let mut gateways = joined(&[A, B]);
        let id = current(&gateways[0]);
        queue(&mut gateways[0], 5, PairMessageKind::Ready, id, &[addr(B)]).unwrap();
        assert_eq!(
            gateways[0].network_keys[&5].pairs[&addr(B)].ready_sent,
            Some(id)
        );

        let message = as_received(&gateways[0].pending_out[0], A);
        assert!(handle_message(&mut gateways[1], &message, &grant_key().verifying_key()).unwrap());
        assert_eq!(
            gateways[1].network_keys[&5].pairs[&addr(A)].peer_ready,
            Some(id)
        );
    }

    #[test]
    fn a_message_for_another_gateway_or_of_another_sender_is_ignored() {
        let mut gateways = joined(&[A, B, C]);
        let id = current(&gateways[0]);

        // a message of a, which only concerns c, doesn't count for the pair of b with a
        let message = PairMessage {
            kind: PairMessageKind::Ready,
            generation: id.0,
            epoch: id.1,
            peers: vec![C.to_string()],
        };
        assert!(!receive(&mut gateways[1], 5, A, &message));

        // c can't speak for a: the sender is the authenticated member of the message
        let message = PairMessage {
            peers: vec![B.to_string()],
            ..message
        };
        assert!(receive(&mut gateways[1], 5, C, &message));
        let pairs = &gateways[1].network_keys[&5].pairs;
        assert_eq!(pairs[&addr(A)].peer_ready, None);
        assert_eq!(pairs[&addr(C)].peer_ready, Some(id));

        // another generation doesn't count either
        let message = PairMessage {
            generation: id.0 + 1,
            ..message
        };
        assert!(!receive(&mut gateways[1], 5, A, &message));
    }

    #[test]
    fn a_replayed_older_message_changes_nothing() {
        let mut gateways = joined(&[A, B]);
        let message = |epoch| PairMessage {
            kind: PairMessageKind::Switched,
            generation: 1,
            epoch,
            peers: vec![B.to_string()],
        };
        assert!(receive(&mut gateways[1], 5, A, &message(3)));
        assert!(!receive(&mut gateways[1], 5, A, &message(2)));
        assert!(!receive(&mut gateways[1], 5, A, &message(3)));
        assert_eq!(
            gateways[1].network_keys[&5].pairs[&addr(A)].peer_switched,
            Some((1, 3))
        );
    }

    #[test]
    fn a_ready_which_arrives_before_its_commit_waits_for_it() {
        let mut gateways = joined(&[A, B]);
        let (a, rest) = gateways.split_first_mut().unwrap();
        let b = &mut rest[0];

        // a rotates the keys and tells b right away, but b gets the ready before the commit
        let commit = stage_update(a, 5).unwrap().unwrap();
        merge_pending(a, 5).unwrap();
        a.apply_commit(5, &CommitChange::default()).unwrap();
        let id = current(a);
        queue(a, 5, PairMessageKind::Ready, id, &[addr(B)]).unwrap();
        let ready = as_received(&a.pending_out[0], A);

        let key = grant_key().verifying_key();
        assert!(handle_message(b, &ready, &key).unwrap());
        assert_eq!(b.pending_in.len(), 1);
        assert_eq!(b.network_keys[&5].pairs[&addr(A)].peer_ready, None);

        deliver(&[commit], b, &mut []);
        b.apply_commit(5, &CommitChange::default()).unwrap();
        assert!(retry_pending(b, &key));
        assert!(b.pending_in.is_empty());
        assert_eq!(b.network_keys[&5].pairs[&addr(A)].peer_ready, Some(id));
    }

    #[test]
    fn a_ready_of_the_last_epoch_is_read_after_the_commit() {
        let mut gateways = joined(&[A, B]);
        let (a, rest) = gateways.split_first_mut().unwrap();
        let b = &mut rest[0];
        let id = current(a);
        queue(a, 5, PairMessageKind::Ready, id, &[addr(B)]).unwrap();
        let ready = as_received(&a.pending_out[0], A);

        // the group moves on, before b reads the ready
        let commit = stage_update(a, 5).unwrap().unwrap();
        merge_pending(a, 5).unwrap();
        deliver(&[commit], b, &mut []);

        assert!(handle_message(b, &ready, &grant_key().verifying_key()).unwrap());
        assert_eq!(b.network_keys[&5].pairs[&addr(A)].peer_ready, Some(id));
    }

    #[tokio::test]
    async fn a_message_stays_queued_until_izakaya_accepted_it() {
        let mut gateways = joined(&[A, B]);
        let a = &mut gateways[0];
        let id = current(a);
        queue(a, 5, PairMessageKind::Ready, id, &[addr(B)]).unwrap();
        queue(a, 5, PairMessageKind::Switched, id, &[addr(B)]).unwrap();

        assert!(!flush(a, |_| async { Err("izakaya is down".to_string()) }).await);
        assert_eq!(a.pending_out.len(), 2);

        // the queue survives a restart
        a.save().unwrap();
        let values = a.provider.storage().values.read().unwrap().clone();
        let mut restored = MlsState::new_in_memory();
        restored.restore(values).unwrap();
        assert_eq!(restored.pending_out.len(), 2);

        let mut sent = Vec::new();
        assert!(
            flush(&mut restored, |message| {
                sent.push(message.payload.clone());
                async { Ok(()) }
            })
            .await
        );
        assert!(restored.pending_out.is_empty());
        assert_eq!(sent.len(), 2);
    }

    /// Kernel of the tests, which accepts every key, or refuses the outgoing ones
    #[derive(Default)]
    struct TestKernel {
        refuse_egress: bool,
    }

    impl PairKernel for TestKernel {
        fn apply(&mut self, _keys: &NetworkKeys) {}

        fn installed(
            &self,
            _keys: &NetworkKeys,
            _peer: Ipv4Addr,
            _id: EpochId,
            direction: CryptoDirection,
        ) -> bool {
            !(self.refuse_egress && direction == CryptoDirection::Egress)
        }
    }

    /// Runs a step of a test-gateway and returns the messages it sends
    fn step_of(gateway: &mut TestGateway) -> Vec<PairMessage> {
        let step = step(&mut gateway.keys, &mut TestKernel::default(), 0);
        step.messages
            .into_iter()
            .map(|(kind, id, peers)| {
                mark_sent(&mut gateway.keys, kind, id, &peers);
                PairMessage {
                    kind,
                    generation: id.0,
                    epoch: id.1,
                    peers: peers.iter().map(|peer| peer.to_string()).collect(),
                }
            })
            .collect()
    }

    fn deliver_to(messages: Vec<PairMessage>, from: &TestGateway, to: &mut TestGateway) {
        let own = to.local_gateway_ip.to_string();
        for message in messages {
            take(&mut to.keys, &own, from.local_gateway_ip, &message);
        }
    }

    /// Removes the old incoming keys of a test-gateway, as if no packet arrives with them
    fn idle(gateway: &mut TestGateway) {
        for _ in 0..2 {
            gateway.check_counters(&mut |_, _, _| Ok(Some(0)));
        }
    }

    fn arrived(gateway: &TestGateway, id: EpochId) -> bool {
        let pair = &gateway.keys.pairs[&gateway.conn.peer_gateway_ip];
        pair.target.is_none() && pair.egress == Some(id)
    }

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

    /// Moves the group of the gateways into a new epoch, which `silent` gateways don't follow
    fn rotate(gateways: &mut [MlsState], silent: &[usize]) {
        let (committer, others) = gateways.split_first_mut().unwrap();
        let commit = stage_update(committer, 5).unwrap().unwrap();
        merge_pending(committer, 5).unwrap();
        committer.apply_commit(5, &CommitChange::default()).unwrap();
        for (i, gateway) in others.iter_mut().enumerate() {
            if silent.contains(&(i + 1)) {
                continue;
            }
            process(gateway, &commit);
            gateway.apply_commit(5, &CommitChange::default()).unwrap();
        }
    }

    fn process(gateway: &mut MlsState, message: &OutgoingMessage) {
        group::process_message(
            gateway,
            message.message_type,
            &message.payload,
            &message.grants,
            &grant_key().verifying_key(),
        )
        .unwrap();
    }

    /// Lets all gateways, which are not `silent`, move their pairs and deliver their messages, and
    /// returns the number of messages, which each gateway sent
    fn drive_all(gateways: &mut [MlsState], silent: &[usize]) -> Vec<usize> {
        let mut sent = vec![0; gateways.len()];
        let mut outgoing = Vec::new();
        for (i, gateway) in gateways.iter_mut().enumerate() {
            if silent.contains(&i) {
                continue;
            }
            drive_pairs(gateway, 5, &mut TestKernel::default(), 0);
            let sender = gateway.client_id().unwrap().to_string();
            sent[i] = gateway.pending_out.len();
            outgoing.extend(gateway.pending_out.drain(..).map(|m| (sender.clone(), m)));
        }
        for (sender, message) in outgoing {
            for (i, gateway) in gateways.iter_mut().enumerate() {
                let id = gateway.client_id().unwrap().to_string();
                if silent.contains(&i) || !message.recipients.contains(&id) {
                    continue;
                }
                handle_message(
                    gateway,
                    &as_received(&message, &sender),
                    &grant_key().verifying_key(),
                )
                .unwrap();
            }
        }
        sent
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

    /// Lets the first gateway commit a change and the others follow it, as the agent does
    fn commit(gateways: &mut [MlsState], additions: Vec<Addition>, removals: &[String]) {
        let (committer, others) = gateways.split_first_mut().unwrap();
        let messages = match additions.is_empty() && removals.is_empty() {
            true => stage_update(committer, 5).unwrap().into_iter().collect(),
            false => stage_change(committer, 5, additions, removals).unwrap(),
        };
        let change = pending_change(committer, 5);
        merge_pending(committer, 5).unwrap();
        committer.apply_commit(5, &change).unwrap();

        for message in &messages {
            for gateway in others.iter_mut() {
                let id = gateway.client_id().unwrap().to_string();
                if !message.recipients.contains(&id) {
                    continue;
                }
                let processed = group::process_message(
                    gateway,
                    message.message_type,
                    &message.payload,
                    &message.grants,
                    &grant_key().verifying_key(),
                )
                .unwrap();
                match processed {
                    Processed::Advanced { change, .. } => {
                        gateway.apply_commit(5, &change).unwrap();
                    }
                    Processed::Joined { .. } => {
                        gateway.reset_epochs(5).unwrap();
                    }
                    _ => {}
                }
            }
        }
    }

    fn pair(gateway: &MlsState, peer: &str) -> PairKeys {
        gateway.network_keys[&5].pairs[&addr(peer)].clone()
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

    #[test]
    fn the_status_tells_the_step_a_pair_waits_for() {
        let mut a = TestGateway::new(1, 1, 2, 2);
        let mut b = TestGateway::new(2, 2, 1, 1);
        let waiting =
            |gateway: &TestGateway| waiting_for(&gateway.keys.pairs[&gateway.conn.peer_gateway_ip]);
        for gateway in [&mut a, &mut b] {
            let pair = gateway.keys.pairs.values_mut().next().unwrap();
            pair.target = None;
        }
        assert_eq!(waiting(&a), MlsPairWaitingFor::Nothing);

        a.record(2);
        assert_eq!(waiting(&a), MlsPairWaitingFor::Ingress);
        let from_a = step_of(&mut a);
        assert_eq!(waiting(&a), MlsPairWaitingFor::PeerReady);

        b.record(2);
        deliver_to(from_a, &a, &mut b);
        b.keys.pairs.values_mut().next().unwrap().peer_ready = Some((1, 2));
        // the switch of b failed, so it waits for its outgoing keys
        let failed = step(
            &mut b.keys,
            &mut TestKernel {
                refuse_egress: true,
            },
            0,
        );
        let mut from_b = Vec::new();
        for (kind, id, peers) in failed.messages {
            mark_sent(&mut b.keys, kind, id, &peers);
            from_b.push(PairMessage {
                kind,
                generation: id.0,
                epoch: id.1,
                peers: peers.iter().map(|peer| peer.to_string()).collect(),
            });
        }
        assert_eq!(waiting(&b), MlsPairWaitingFor::Egress);
        from_b.extend(step_of(&mut b));
        assert_eq!(waiting(&b), MlsPairWaitingFor::PeerSwitched);

        deliver_to(from_b, &b, &mut a);
        step_of(&mut a);
        assert_eq!(waiting(&a), MlsPairWaitingFor::OldIngressInUse);
        idle(&mut a);
        assert_eq!(waiting(&a), MlsPairWaitingFor::Nothing);
    }

    #[test]
    fn the_status_lists_the_pairs_of_every_network() {
        let gateways = joined(&[A, B]);
        let status = status(&gateways[0]);
        assert_eq!(status.client_id.as_deref(), Some(A));
        assert_eq!(status.networks.len(), 1);
        let network = &status.networks[0];
        assert_eq!(network.vni, 5);
        assert_eq!(network.generation, 1);
        assert_eq!(network.epoch, Some(current(&gateways[0]).1));
        assert_eq!(network.pairs.len(), 1);
        let pair = &network.pairs[0];
        assert_eq!(pair.peer, B);
        assert_eq!(pair.waiting_for, MlsPairWaitingFor::Ingress);
        assert_eq!(pair.egress, pair.target);
        assert!(pair.last_error.is_none());
    }
}
