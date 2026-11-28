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

//! MLS-messages between the gateways, which travel over the izakaya.

use crate::core::mls_key_exchange::commit::commit_change;
use crate::core::mls_key_exchange::commit::{CommitChange, verify_additions};
use crate::core::mls_key_exchange::group::join_config;
use crate::core::mls_key_exchange::group::{encode, internal_error, own_identity};
use crate::core::mls_key_exchange::identity::{identity_of, vni_of};
use crate::core::mls_key_exchange::state::MlsState;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::{MlsGrant, MlsMessageType, VerifyingKey};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openmls::prelude::{tls_codec::*, *};

/// Message, which has to be delivered to other gateways over the izakaya
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutgoingMessage {
    pub group_id: String,
    /// Epoch of the group, which the message leads into
    pub epoch: u64,
    pub message_type: MlsMessageType,
    pub recipients: Vec<String>,
    /// Base64-encoded TLS-serialized MLS-message
    pub payload: String,
    /// Grants of the gateways, which a commit adds, so the other members can check them
    pub grants: Vec<MlsGrant>,
}

/// Encrypts a message for other members of the group of a network.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `vni` - Tenant of the network
/// * `content` - The plaintext
/// * `recipients` - Identities of the members, which get the message
///
/// # Returns
/// The message for the recipients, or the error of openmls
pub fn create_application_message(
    state: &mut MlsState,
    vni: u32,
    content: &[u8],
    recipients: Vec<String>,
) -> Result<OutgoingMessage, ErrorResponse> {
    let identity = own_identity(&state.identity)?;
    let group = state
        .groups
        .get_mut(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
    let message = group
        .create_message(&state.provider, &identity.signer, content)
        .map_err(|e| internal_error("encrypt a message for the MLS-group", e))?;
    Ok(OutgoingMessage {
        group_id: String::from_utf8_lossy(group.group_id().as_slice()).into_owned(),
        epoch: group.epoch().as_u64(),
        message_type: MlsMessageType::Application,
        recipients,
        payload: encode(&message)?,
        grants: Vec::new(),
    })
}

/// Effect of an incoming message on the groups of the gateway
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Processed {
    /// The gateway joined the group of a network. `replaced` tells, that it already had an
    /// older group of the same network, whose keys are worthless now.
    Joined { vni: u32, replaced: bool },
    /// The group of a network moved into its next epoch
    Advanced { vni: u32, change: CommitChange },
    /// The gateway was removed from the group of a network
    Left { vni: u32 },
    /// A member sent a message to this gateway
    Application {
        vni: u32,
        /// Identity of the member, which sent the message
        sender: String,
        content: Vec<u8>,
    },
    /// The message belongs to an epoch, which the group didn't reach yet, so it has to wait for
    /// the commit into that epoch
    Pending,
    /// The message didn't change anything
    Ignored,
}

/// Processes a message, which another gateway sent over the izakaya.
///
/// A welcome lets the gateway join the group of a network, a commit moves a group, which the
/// gateway is already a member of, into its next epoch, and an application message is a message
/// of the peer of a pair.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `message_type` - Type of the message, as told by the izakaya
/// * `payload` - Base64-encoded TLS-serialized MLS-message
/// * `grants` - Grants, which are attached to the message
/// * `grant_key` - Public key of hanami, which signs the grants
///
/// # Returns
/// The effect of the message, or the reason why it couldn't be processed
pub fn process_message(
    state: &mut MlsState,
    message_type: MlsMessageType,
    payload: &str,
    grants: &[MlsGrant],
    grant_key: &VerifyingKey,
) -> Result<Processed, String> {
    let bytes = BASE64
        .decode(payload)
        .map_err(|e| format!("message is not base64-encoded: {e}"))?;
    let message = MlsMessageIn::tls_deserialize_exact(bytes.as_slice())
        .map_err(|e| format!("message can not be decoded: {e}"))?;

    match message_type {
        MlsMessageType::Welcome => process_welcome(state, message),
        MlsMessageType::Commit => process_commit(state, message, grants, grant_key),
        MlsMessageType::Application => process_application(state, message),
        MlsMessageType::Operation => Err(format!("a {message_type} is no MLS-message")),
    }
}

/// Joins the group of a network with a welcome. An older group of the same network is a
/// leftover of an earlier membership, which the invitation supersedes.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `message` - The welcome
///
/// # Returns
/// `Joined`, or the reason why the welcome couldn't be processed
fn process_welcome(state: &mut MlsState, message: MlsMessageIn) -> Result<Processed, String> {
    let welcome = match message.extract() {
        MlsMessageBodyIn::Welcome(welcome) => welcome,
        _ => return Err("message is not a welcome".to_string()),
    };
    let staged = StagedWelcome::build_from_welcome(&state.provider, &join_config(), welcome)
        .map_err(|e| format!("welcome can not be processed: {e}"))?
        .replace_old_group()
        .build()
        .map_err(|e| format!("welcome can not be processed: {e}"))?;
    let vni = vni_of(staged.group_context().group_id())
        .ok_or_else(|| "welcome belongs to an unknown group".to_string())?;
    let group = staged
        .into_group(&state.provider)
        .map_err(|e| format!("group of the welcome can not be joined: {e}"))?;

    let replaced = state.groups.insert(vni, group).is_some();
    Ok(Processed::Joined { vni, replaced })
}

/// Moves a group into its next epoch with a commit of another member.
///
/// A commit for an epoch, which the group has already left, is ignored, because it was processed
/// already. Every gateway, which a commit adds, has to be covered by a grant of hanami, which is
/// attached to the commit. A commit, which adds a gateway without grant, is refused, so not even
/// a compromised committer can bring a gateway into the group, which hanami didn't allow.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `message` - The commit
/// * `grants` - Grants, which are attached to the commit
/// * `grant_key` - Public key of hanami
///
/// # Returns
/// `Advanced` with the change of the commit, `Left` if the commit removed this gateway, or the
/// reason why the commit is refused
fn process_commit(
    state: &mut MlsState,
    message: MlsMessageIn,
    grants: &[MlsGrant],
    grant_key: &VerifyingKey,
) -> Result<Processed, String> {
    let message = message
        .try_into_protocol_message()
        .map_err(|e| format!("message is not a commit: {e}"))?;
    let vni = vni_of(message.group_id())
        .ok_or_else(|| "commit belongs to an unknown group".to_string())?;
    let Some(group) = state.groups.get_mut(&vni) else {
        log::warn!("Ignore commit for the MLS-group of tenant {vni}: not a member");
        return Ok(Processed::Ignored);
    };
    if message.epoch() < group.epoch() {
        log::debug!(
            "Ignore commit of epoch {} for the MLS-group of tenant {vni}, which is in epoch {} \
             already",
            message.epoch().as_u64(),
            group.epoch().as_u64()
        );
        return Ok(Processed::Ignored);
    }

    let processed = group
        .process_message(&state.provider, message)
        .map_err(|e| format!("commit can not be processed: {e}"))?;
    let committer = identity_of(processed.credential());
    let staged_commit = match processed.into_content() {
        ProcessedMessageContent::StagedCommitMessage(staged_commit) => staged_commit,
        _ => return Ok(Processed::Ignored),
    };
    if let Err(e) = verify_additions(&staged_commit, vni, grants, grant_key) {
        // the group stays in its epoch and keeps using the keys of it
        if let Err(clear_err) = group.clear_pending_commit(state.provider.storage()) {
            log::error!("Failed to drop the refused commit of tenant {vni}: {clear_err}");
        }
        return Err(format!("commit is refused: {e}"));
    }
    let change = commit_change(group, &staged_commit, committer);
    group
        .merge_staged_commit(&state.provider, *staged_commit)
        .map_err(|e| format!("commit can not be merged: {e}"))?;

    if !group.is_active() {
        drop_left_group(state, vni);
        return Ok(Processed::Left { vni });
    }
    Ok(Processed::Advanced { vni, change })
}

/// Drops the group of a network, after a commit removed this gateway from it, because it has no
/// use for the group anymore.
fn drop_left_group(state: &mut MlsState, vni: u32) {
    if let Some(mut group) = state.groups.remove(&vni)
        && let Err(e) = group.delete(state.provider.storage())
    {
        log::error!("Failed to delete the left MLS-group of tenant {vni}: {e}");
    }
}

/// Decrypts an application message of another member. A message of an epoch, which the group
/// didn't reach yet, has to wait for the commit into that epoch.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `message` - The application message
///
/// # Returns
/// `Application` with the sender and the plaintext, `Pending` for a message of a future epoch, or
/// the reason why the message can't be decrypted
fn process_application(state: &mut MlsState, message: MlsMessageIn) -> Result<Processed, String> {
    let message = message
        .try_into_protocol_message()
        .map_err(|e| format!("message is no application message: {e}"))?;
    let vni = vni_of(message.group_id())
        .ok_or_else(|| "message belongs to an unknown group".to_string())?;
    let Some(group) = state.groups.get_mut(&vni) else {
        log::debug!("Ignore message for the MLS-group of tenant {vni}: not a member");
        return Ok(Processed::Ignored);
    };
    if message.epoch() > group.epoch() {
        return Ok(Processed::Pending);
    }

    let processed = group
        .process_message(&state.provider, message)
        .map_err(|e| format!("message can not be processed: {e}"))?;
    let sender = identity_of(processed.credential())
        .ok_or_else(|| "the sender has no basic credential".to_string())?;
    match processed.into_content() {
        ProcessedMessageContent::ApplicationMessage(content) => Ok(Processed::Application {
            vni,
            sender,
            content: content.into_bytes(),
        }),
        _ => Err("message is no application message".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::commit::{merge_pending, stage_change};
    use crate::core::mls_key_exchange::group::{create_group, encode};
    use crate::core::mls_key_exchange::identity::identity_of;
    use crate::core::mls_key_exchange::state::MlsState;
    use crate::core::mls_key_exchange::test_support::{
        addition, addition_with_grant, deliver, grant_for, grant_key, invite, new_gateway, secret,
    };
    use ainari_api_structs::mls_structs::SigningKey;

    #[test]
    fn a_commit_of_an_old_epoch_is_ignored() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        let addition = addition(&a, &c, 5);
        let messages = stage_change(&mut a, 5, vec![addition], &[]).unwrap();
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut c, &mut [&mut b]);

        // the same commit a second time doesn't move the group again
        let commit = &messages[0];
        assert_eq!(
            process_message(
                &mut b,
                commit.message_type,
                &commit.payload,
                &commit.grants,
                &grant_key().verifying_key()
            )
            .unwrap(),
            Processed::Ignored
        );
        assert_eq!(secret(&a, 5), secret(&b, 5));
    }

    #[test]
    fn the_removed_gateway_leaves_the_group_with_the_commit() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        // the commit is not addressed to the removed gateway, but if it gets it anyway, it
        // drops the group
        let group = a.groups.get_mut(&5).unwrap();
        let leaf = group
            .members()
            .find(|member| identity_of(&member.credential).as_deref() == Some("10.0.0.2"))
            .unwrap()
            .index;
        let identity = a.identity.as_ref().unwrap();
        let (commit, _, _) = group
            .remove_members(&a.provider, &identity.signer, &[leaf])
            .unwrap();
        merge_pending(&mut a, 5).unwrap();

        let payload = encode(&commit).unwrap();
        assert_eq!(
            process_message(
                &mut b,
                MlsMessageType::Commit,
                &payload,
                &[],
                &grant_key().verifying_key()
            )
            .unwrap(),
            Processed::Left { vni: 5 }
        );
        assert!(b.groups.is_empty());
    }

    /// Lets `a` add `c` with the given grant and returns the commit for the other members.
    fn add_with_grant(a: &mut MlsState, c: &MlsState, grant: &MlsGrant) -> OutgoingMessage {
        let mut addition = addition(a, c, 5);
        addition.grant = grant.clone();
        let messages = stage_change(a, 5, vec![addition], &[]).unwrap();
        merge_pending(a, 5).unwrap();
        messages
            .into_iter()
            .find(|message| message.message_type == MlsMessageType::Commit)
            .unwrap()
    }

    #[test]
    fn a_member_refuses_an_addition_without_grant() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let c = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        let epoch = b.groups[&5].epoch();

        // a compromised committer adds c with a grant, which hanami didn't sign
        let forged = MlsGrant::sign(
            &grant_for(&c, 5)
                .verify(&grant_key().verifying_key())
                .unwrap(),
            &SigningKey::from_bytes(&[6u8; 32]),
        );
        let commit = add_with_grant(&mut a, &c, &forged);
        let result = process_message(
            &mut b,
            commit.message_type,
            &commit.payload,
            &commit.grants,
            &grant_key().verifying_key(),
        );
        assert!(result.is_err());
        // b stays in its epoch and keeps its keys
        assert_eq!(b.groups[&5].epoch(), epoch);
    }

    #[test]
    fn a_grant_for_another_key_does_not_cover_an_addition() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let c = new_gateway("10.0.0.3");
        // the impostor claims the identity of c with a key of its own
        let impostor = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        let commit = add_with_grant(&mut a, &impostor, &grant_for(&c, 5));
        let result = process_message(
            &mut b,
            commit.message_type,
            &commit.payload,
            &commit.grants,
            &grant_key().verifying_key(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn a_member_refuses_a_batch_with_one_addition_without_grant() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let c = new_gateway("10.0.0.3");
        let d = new_gateway("10.0.0.4");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        let epoch = b.groups[&5].epoch();

        // d comes with the grant of c, so only c is covered
        let additions = vec![addition(&a, &c, 5), addition_with_grant(&a, &d, &c, 5)];
        let messages = stage_change(&mut a, 5, additions, &[]).unwrap();
        merge_pending(&mut a, 5).unwrap();

        let commit = &messages[0];
        let result = process_message(
            &mut b,
            commit.message_type,
            &commit.payload,
            &commit.grants,
            &grant_key().verifying_key(),
        );
        assert!(result.is_err());
        assert_eq!(b.groups[&5].epoch(), epoch);
    }
}
