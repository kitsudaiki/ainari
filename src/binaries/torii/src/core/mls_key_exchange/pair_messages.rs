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

//! `Ready` and `Switched`: the messages between the two gateways of a pair.
//!
//! The messages are MLS application messages: encrypted within the group and signed by the leaf
//! of the sender, so izakaya can neither read nor forge them, and a member can only speak for
//! its own pairs.

use crate::core::mls_key_exchange::group::{self, MAX_PAST_EPOCHS};
use crate::core::mls_key_exchange::identity::vni_of;
use crate::core::mls_key_exchange::messages::{self, OutgoingMessage, Processed};
use crate::core::mls_key_exchange::network_keys::{EpochId, NetworkKeys};
use crate::core::mls_key_exchange::state::MlsState;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::{MlsMessageResp, MlsMessageType, VerifyingKey};
use openmls::prelude::GroupId;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

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
    let outgoing = messages::create_application_message(state, vni, &content, recipients)?;
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
    let processed = messages::process_message(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::commit::{CommitChange, merge_pending, stage_update};
    use crate::core::mls_key_exchange::state::MlsState;
    use crate::core::mls_key_exchange::test_support::{
        A, B, C, addr, as_received, current, deliver, grant_key, joined,
    };

    use openmls::prelude::OpenMlsProvider;

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
}
