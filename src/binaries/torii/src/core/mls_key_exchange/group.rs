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

//! Operations on the MLS-groups of the gateway.
//!
//! Nothing in here talks to the izakaya or to the database: an operation, which changes a group,
//! is only staged and hands back the messages, which the other gateways need to follow it. The
//! caller delivers them and merges the staged change only afterwards, so a group never moves
//! into an epoch, which the other members can't reach.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openmls::prelude::{tls_codec::*, *};

use crate::core::mls_key_exchange::state::{
    CIPHERSUITE, MlsIdentity, MlsState, group_id_of, identity_of, vni_of,
};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::{
    MlsGrant, MlsGrantAction, MlsGrantPayload, MlsMessageType, MlsOperationAdd,
    MlsOperationRefusal, VerifyingKey,
};

/// Number of past epochs of a group, whose messages can still be decrypted, so a message, which
/// was encrypted shortly before a commit, can still be read after it
pub const MAX_PAST_EPOCHS: usize = 16;

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

/// Logs an error of openmls and turns it into the generic `InternalError`.
pub fn internal_error(action: &str, e: impl std::fmt::Debug) -> ErrorResponse {
    ainari_api::report_internal_error!(None, "Failed to {action}: {e:?}");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Returns the identity of the gateway, which `MlsState::ensure_identity` has to create before.
pub fn own_identity(identity: &Option<MlsIdentity>) -> Result<&MlsIdentity, ErrorResponse> {
    identity.as_ref().ok_or_else(|| {
        ainari_api::report_internal_error!(
            None,
            "The MLS-identity of the gateway was not created yet"
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })
}

/// Serializes an outgoing MLS-message for the izakaya.
pub fn encode(message: &MlsMessageOut) -> Result<String, ErrorResponse> {
    message
        .tls_serialize_detached()
        .map(|bytes| BASE64.encode(bytes))
        .map_err(|e| internal_error("serialize a MLS-message", e))
}

/// Lists the identities of all members of a group, which have to follow a change of it.
///
/// # Arguments
/// * `group` - The group
/// * `own_client_id` - Identity of this gateway, which made the change itself
/// * `excluded` - Identities of the members, which are removed or re-added with the change
///
/// # Returns
/// The identities of the other members
pub fn other_members(group: &MlsGroup, own_client_id: &str, excluded: &[&str]) -> Vec<String> {
    let mut members: Vec<String> = group
        .members()
        .filter_map(|member| identity_of(&member.credential))
        .filter(|id| id != own_client_id && !excluded.contains(&id.as_str()))
        .collect();
    members.sort();
    members.dedup();
    members
}

/// Moves the group of a network into the epoch of its staged change.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// `Ok(())` once the group is in its new epoch
pub fn merge_pending(state: &mut MlsState, vni: u32) -> Result<(), ErrorResponse> {
    let group = state
        .groups
        .get_mut(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
    group
        .merge_pending_commit(&state.provider)
        .map_err(|e| internal_error("merge the staged change of the MLS-group", e))
}

/// Drops the staged change of the group of a network, after it couldn't be delivered.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
pub fn clear_pending(state: &mut MlsState, vni: u32) {
    if let Some(group) = state.groups.get_mut(&vni)
        && let Err(e) = group.clear_pending_commit(state.provider.storage())
    {
        log::error!("Failed to drop the staged change of the MLS-group of tenant {vni}: {e}");
    }
}

/// Creates new key-packages of the gateway, which another gateway uses to invite it.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `count` - Number of key-packages
///
/// # Returns
/// The base64-encoded key-packages
pub fn create_key_packages(state: &MlsState, count: u32) -> Result<Vec<String>, ErrorResponse> {
    let identity = own_identity(&state.identity)?;

    (0..count)
        .map(|_| {
            let bundle = KeyPackage::builder()
                .build(
                    CIPHERSUITE,
                    &state.provider,
                    &identity.signer,
                    identity.credential.clone(),
                )
                .map_err(|e| internal_error("create a key-package", e))?;
            bundle
                .key_package()
                .tls_serialize_detached()
                .map(|bytes| BASE64.encode(bytes))
                .map_err(|e| internal_error("serialize a key-package", e))
        })
        .collect()
}

/// Configuration of a new group. The ratchet-tree is part of every welcome, so an invited
/// gateway needs nothing else than the welcome to join.
fn create_config() -> MlsGroupCreateConfig {
    MlsGroupCreateConfig::builder()
        .ciphersuite(CIPHERSUITE)
        .use_ratchet_tree_extension(true)
        .max_past_epochs(MAX_PAST_EPOCHS)
        .build()
}

/// Creates the group of a network with this gateway as its only member.
///
/// An already existing group of the network is replaced, because the control plane only creates
/// a group, if no other gateway serves the network anymore.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `vni` - Tenant of the network
///
/// # Returns
/// `true` if an older group of the network was replaced, otherwise `false`
pub fn create_group(state: &mut MlsState, vni: u32) -> Result<bool, ErrorResponse> {
    let replaced = match state.groups.remove(&vni) {
        Some(mut old_group) => {
            old_group
                .delete(state.provider.storage())
                .map_err(|e| internal_error("delete the old MLS-group", e))?;
            true
        }
        None => false,
    };

    let identity = own_identity(&state.identity)?;
    let group = MlsGroup::new_with_group_id(
        &state.provider,
        &identity.signer,
        &create_config(),
        group_id_of(vni),
        identity.credential.clone(),
    )
    .map_err(|e| internal_error("create the MLS-group", e))?;

    state.groups.insert(vni, group);
    Ok(replaced)
}

/// Removes the group of a network from the gateway.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// `Ok(())`, or `NotFound` if the gateway is not a member of the group
pub fn delete_group(state: &mut MlsState, vni: u32) -> Result<(), ErrorResponse> {
    let mut group = state
        .groups
        .remove(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
    group
        .delete(state.provider.storage())
        .map_err(|e| internal_error("delete the MLS-group", e))
}

/// Decodes and validates a key-package, which was claimed from the izakaya.
///
/// The izakaya validates the key-packages already, but it is not trusted with the decision, who
/// is invited: the identity in the credential has to be the gateway, which should be invited.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `client_id` - Identity of the gateway, which should be invited
/// * `encoded` - Base64-encoded TLS-serialized key-package
///
/// # Returns
/// The validated key-package, or `BadRequest` if it is not usable
pub fn decode_key_package(
    state: &MlsState,
    client_id: &str,
    encoded: &str,
) -> Result<KeyPackage, ErrorResponse> {
    let invalid = |reason: String| {
        ErrorResponse::BadRequest(format!("Invalid key-package of '{client_id}': {reason}"))
    };

    let bytes = BASE64.decode(encoded).map_err(|e| invalid(e.to_string()))?;
    let key_package = KeyPackageIn::tls_deserialize_exact(bytes.as_slice())
        .map_err(|e| invalid(e.to_string()))?
        .validate(state.provider.crypto(), ProtocolVersion::Mls10)
        .map_err(|e| invalid(e.to_string()))?;

    match identity_of(key_package.leaf_node().credential()) {
        Some(identity) if identity == client_id => Ok(key_package),
        Some(identity) => Err(invalid(format!("it belongs to '{identity}'"))),
        None => Err(invalid("it has no basic credential".to_string())),
    }
}

/// Checks, if a gateway is a member of the group of a network.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
/// * `client_id` - Identity of the gateway
///
/// # Returns
/// `true` if the gateway is a member
pub fn is_member(state: &MlsState, vni: u32, client_id: &str) -> bool {
    state.groups.get(&vni).is_some_and(|group| {
        group
            .members()
            .any(|member| identity_of(&member.credential).as_deref() == Some(client_id))
    })
}

/// Gateway, which a change adds to the group of a network
#[derive(Debug, Clone)]
pub struct Addition {
    /// Identity of the added gateway
    pub client_id: String,
    /// Validated key-package of the added gateway
    pub key_package: KeyPackage,
    /// Grant of hanami for the added gateway, which is attached to the commit, so the other
    /// members can check it
    pub grant: MlsGrant,
}

/// Checks the gateways, which a change should add to the group of a network, and claims their
/// key-packages.
///
/// A gateway is only added with a valid grant of hanami, which names exactly the identity and
/// the signature-key of its key-package. A gateway, which fails a check, is left out, the others
/// are added anyway.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
/// * `adds` - The gateways to add, with their grants
/// * `grant_key` - Public key of hanami
/// * `now` - Current unix-time in seconds
/// * `claim` - Claims a base64-encoded key-package of a gateway with the granted signature-key
///   (arguments: identity, signature-key)
///
/// # Returns
/// The checked additions and the gateways, which were left out
pub async fn check_additions<F, Fut>(
    state: &MlsState,
    vni: u32,
    adds: &[MlsOperationAdd],
    grant_key: &VerifyingKey,
    now: i64,
    mut claim: F,
) -> (Vec<Addition>, Vec<MlsOperationRefusal>)
where
    F: FnMut(String, String) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let mut additions = Vec::new();
    let mut refused = Vec::new();
    for add in adds {
        let target = add.client_id.as_str();
        let checked = async {
            let payload = verify_grant(&add.grant, grant_key, vni, target)?;
            if payload.is_expired_at(now) {
                return Err("the grant is expired".to_string());
            }
            let encoded = claim(target.to_string(), payload.signature_key.clone()).await?;
            let key_package =
                decode_key_package(state, target, &encoded).map_err(|e| format!("{e:?}"))?;
            let signature_key = BASE64.encode(key_package.leaf_node().signature_key().as_slice());
            if signature_key != payload.signature_key {
                return Err(format!(
                    "the key-package of '{target}' has another signature-key than granted"
                ));
            }
            Ok(key_package)
        }
        .await;

        match checked {
            Ok(key_package) => additions.push(Addition {
                client_id: target.to_string(),
                key_package,
                grant: add.grant.clone(),
            }),
            Err(reason) => refused.push(MlsOperationRefusal {
                client_id: target.to_string(),
                reason,
            }),
        }
    }
    (additions, refused)
}

/// Stages one change of the group of a network, which adds and removes any number of gateways
/// with a single commit.
///
/// A gateway, which is added, while it is still a member from an earlier membership (for
/// example, because it lost the group), loses its old leaf with the same commit.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `vni` - Tenant of the network
/// * `additions` - Gateways to add
/// * `removals` - Identities of the gateways to remove
///
/// # Returns
/// The commit for the remaining members, if there are any, followed by the welcome for the added
/// gateways, if there are any. `BadRequest` if the change is empty or removes this gateway
/// itself, `NotFound` if a removed gateway is not a member.
pub fn stage_change(
    state: &mut MlsState,
    vni: u32,
    additions: Vec<Addition>,
    removals: &[String],
) -> Result<Vec<OutgoingMessage>, ErrorResponse> {
    if additions.is_empty() && removals.is_empty() {
        return Err(ErrorResponse::BadRequest(
            "A change has to add or remove a gateway".to_string(),
        ));
    }
    let identity = own_identity(&state.identity)?;
    if removals.contains(&identity.client_id)
        || additions
            .iter()
            .any(|it| it.client_id == identity.client_id)
    {
        return Err(ErrorResponse::BadRequest(
            "A gateway can't remove or add itself, delete its group instead".to_string(),
        ));
    }

    let group = state
        .groups
        .get_mut(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;

    for client_id in removals {
        if !group
            .members()
            .any(|member| identity_of(&member.credential).as_ref() == Some(client_id))
        {
            return Err(ErrorResponse::NotFound(format!(
                "'{client_id}' is not a member of the MLS-group of tenant {vni}"
            )));
        }
    }

    // the old leaves of the re-added gateways go together with the removed ones
    let mut excluded: Vec<&str> = removals.iter().map(String::as_str).collect();
    excluded.extend(additions.iter().map(|it| it.client_id.as_str()));
    let leaves: Vec<LeafNodeIndex> = group
        .members()
        .filter(|member| {
            identity_of(&member.credential).is_some_and(|id| excluded.contains(&id.as_str()))
        })
        .map(|member| member.index)
        .collect();

    let recipients = other_members(group, &identity.client_id, &excluded);
    let invited: Vec<String> = additions.iter().map(|it| it.client_id.clone()).collect();
    let grants: Vec<MlsGrant> = additions.iter().map(|it| it.grant.clone()).collect();

    let bundle = group
        .commit_builder()
        .propose_removals(leaves)
        .propose_adds(additions.into_iter().map(|it| it.key_package))
        .force_self_update(true)
        .load_psks(state.provider.storage())
        .map_err(|e| internal_error("load the PSKs of the MLS-group", e))?
        .build(
            state.provider.rand(),
            state.provider.crypto(),
            &identity.signer,
            |_| true,
        )
        .map_err(|e| internal_error("build the commit of the MLS-group", e))?
        .stage_commit(&state.provider)
        .map_err(|e| internal_error("stage the commit of the MLS-group", e))?;
    let (commit, welcome, _) = bundle.into_messages();

    let group_id = String::from_utf8_lossy(group.group_id().as_slice()).into_owned();
    let epoch = group.epoch().as_u64() + 1;

    let mut messages = Vec::new();
    if !recipients.is_empty() {
        messages.push(OutgoingMessage {
            group_id: group_id.clone(),
            epoch,
            message_type: MlsMessageType::Commit,
            recipients,
            payload: encode(&commit)?,
            grants: grants.clone(),
        });
    }
    if let Some(welcome) = welcome {
        messages.push(OutgoingMessage {
            group_id,
            epoch,
            message_type: MlsMessageType::Welcome,
            recipients: invited,
            payload: encode(&welcome)?,
            grants,
        });
    }
    Ok(messages)
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

/// The gateways, which a commit adds and removes, as the content of the commit tells it. MLS
/// authenticates the content, so izakaya can't change it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitChange {
    /// Identity of the member, which made the commit
    pub committer: Option<String>,
    /// Identities of the added gateways, including the re-added ones
    pub added: Vec<String>,
    /// Identities of the removed gateways, including the re-added ones, whose old leaf goes
    pub removed: Vec<String>,
}

impl CommitChange {
    /// Lists the gateways, which the commit removes for good.
    pub fn gone(&self) -> Vec<&String> {
        self.removed
            .iter()
            .filter(|removed| !self.added.contains(removed))
            .collect()
    }

    /// Checks, if the commit has to move every pair to its new epoch: because it removes a
    /// gateway, which must not use any of the keys afterwards, or because it adds and removes
    /// nobody, which is the regular key-rotation. A commit, which only adds gateways, only moves
    /// their pairs, so a new gateway can't derive the keys between the others.
    pub fn moves_all(&self) -> bool {
        !self.gone().is_empty() || (self.added.is_empty() && self.removed.is_empty())
    }
}

/// Reads the gateways, which a staged commit adds and removes.
///
/// # Arguments
/// * `group` - The group, before the commit is merged
/// * `staged_commit` - The commit
/// * `committer` - Identity of the member, which made the commit
///
/// # Returns
/// The change of the commit
fn commit_change(
    group: &MlsGroup,
    staged_commit: &StagedCommit,
    committer: Option<String>,
) -> CommitChange {
    let mut change = CommitChange {
        committer,
        added: staged_commit
            .add_proposals()
            .filter_map(|addition| {
                identity_of(
                    addition
                        .add_proposal()
                        .key_package()
                        .leaf_node()
                        .credential(),
                )
            })
            .collect(),
        removed: staged_commit
            .remove_proposals()
            .filter_map(|removal| group.member(removal.remove_proposal().removed()))
            .filter_map(identity_of)
            .collect(),
    };
    change.added.sort();
    change.added.dedup();
    change.removed.sort();
    change.removed.dedup();
    change
}

/// Reads the gateways, which the staged change of this gateway adds and removes.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// The change, or a pure self-update, if nothing is staged
pub fn pending_change(state: &MlsState, vni: u32) -> CommitChange {
    let committer = state.client_id().map(str::to_string);
    match state
        .groups
        .get(&vni)
        .and_then(|group| group.pending_commit().map(|commit| (group, commit)))
    {
        Some((group, commit)) => commit_change(group, commit, committer),
        None => CommitChange {
            committer,
            ..Default::default()
        },
    }
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

/// Configuration of a group, which is joined by a welcome
fn join_config() -> MlsGroupJoinConfig {
    MlsGroupJoinConfig::builder()
        .use_ratchet_tree_extension(true)
        .max_past_epochs(MAX_PAST_EPOCHS)
        .build()
}

/// Processes a message, which another gateway sent over the izakaya.
///
/// A welcome lets the gateway join the group of a network, a commit moves a group, which the
/// gateway is already a member of, into its next epoch. A commit for an epoch, which the group
/// has already left, is ignored, because it was processed already.
///
/// Every gateway, which a commit adds, has to be covered by a grant of hanami, which is attached
/// to the commit. A commit, which adds a gateway without grant, is refused, so not even a
/// compromised committer can bring a gateway into the group, which hanami didn't allow.
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
        MlsMessageType::Welcome => {
            let welcome = match message.extract() {
                MlsMessageBodyIn::Welcome(welcome) => welcome,
                _ => return Err("message is not a welcome".to_string()),
            };

            // an older group of the same network is a leftover of an earlier membership, which
            // the invitation supersedes
            let staged =
                StagedWelcome::build_from_welcome(&state.provider, &join_config(), welcome)
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
        MlsMessageType::Operation => Err(format!("a {message_type} is no MLS-message")),
        MlsMessageType::Application => {
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
                ProcessedMessageContent::ApplicationMessage(content) => {
                    Ok(Processed::Application {
                        vni,
                        sender,
                        content: content.into_bytes(),
                    })
                }
                _ => Err("message is no application message".to_string()),
            }
        }
        MlsMessageType::Commit => {
            let message = message
                .try_into_protocol_message()
                .map_err(|e| format!("message is not a commit: {e}"))?;
            let vni = vni_of(message.group_id())
                .ok_or_else(|| "commit belongs to an unknown group".to_string())?;

            let group = match state.groups.get_mut(&vni) {
                Some(group) => group,
                None => {
                    log::warn!("Ignore commit for the MLS-group of tenant {vni}: not a member");
                    return Ok(Processed::Ignored);
                }
            };
            if message.epoch() < group.epoch() {
                log::debug!(
                    "Ignore commit of epoch {} for the MLS-group of tenant {vni}, which is in \
                     epoch {} already",
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

            // a gateway, which was removed with the commit, has no use for the group anymore
            if !group.is_active() {
                if let Some(mut group) = state.groups.remove(&vni)
                    && let Err(e) = group.delete(state.provider.storage())
                {
                    log::error!("Failed to delete the left MLS-group of tenant {vni}: {e}");
                }
                return Ok(Processed::Left { vni });
            }

            Ok(Processed::Advanced { vni, change })
        }
    }
}

/// Checks the grant of a gateway, which should be added to the group of a network.
///
/// # Arguments
/// * `grant` - The grant
/// * `grant_key` - Public key of hanami
/// * `vni` - Tenant of the network
/// * `client_id` - Identity of the gateway
///
/// # Returns
/// The content of the grant, or the reason, why it doesn't allow the gateway to join
pub fn verify_grant(
    grant: &MlsGrant,
    grant_key: &VerifyingKey,
    vni: u32,
    client_id: &str,
) -> Result<MlsGrantPayload, String> {
    let payload = grant.verify(grant_key)?;
    if payload.action != MlsGrantAction::Add {
        return Err("the grant doesn't allow to join".to_string());
    }
    if payload.vni != vni || payload.client_id != client_id {
        return Err(format!(
            "the grant is for '{}' in tenant {}",
            payload.client_id, payload.vni
        ));
    }
    Ok(payload)
}

/// Checks, that every gateway, which a commit adds, is covered by a grant of hanami.
///
/// The grant has to name the identity and the signature-key of the added member. Its expiration
/// is not checked here, because a member can't tell, when the committer made the commit.
///
/// # Arguments
/// * `staged_commit` - The commit
/// * `vni` - Tenant of the network
/// * `grants` - Grants, which are attached to the commit
/// * `grant_key` - Public key of hanami
///
/// # Returns
/// `Ok(())` if all additions are allowed, otherwise the first one, which is not
pub fn verify_additions(
    staged_commit: &StagedCommit,
    vni: u32,
    grants: &[MlsGrant],
    grant_key: &VerifyingKey,
) -> Result<(), String> {
    for addition in staged_commit.add_proposals() {
        let leaf = addition.add_proposal().key_package().leaf_node();
        let identity = identity_of(leaf.credential())
            .ok_or_else(|| "an added member has no basic credential".to_string())?;
        let signature_key = BASE64.encode(leaf.signature_key().as_slice());

        let covered = grants.iter().any(|grant| {
            verify_grant(grant, grant_key, vni, &identity)
                .is_ok_and(|payload| payload.signature_key == signature_key)
        });
        if !covered {
            return Err(format!("'{identity}' is added without a grant of hanami"));
        }
    }
    Ok(())
}

/// Stages a key-rotation of the group of a network: the gateway renews its own leaf, which moves
/// the group into a new epoch with a fresh secret.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `vni` - Tenant of the network
///
/// # Returns
/// The commit for the other members, or `None` if there are no other ones
pub fn stage_update(
    state: &mut MlsState,
    vni: u32,
) -> Result<Option<OutgoingMessage>, ErrorResponse> {
    let identity = own_identity(&state.identity)?;
    let group = state
        .groups
        .get_mut(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;

    let recipients = other_members(group, &identity.client_id, &[]);
    let bundle = group
        .self_update(
            &state.provider,
            &identity.signer,
            LeafNodeParameters::default(),
        )
        .map_err(|e| internal_error("rotate the keys of the MLS-group", e))?;

    if recipients.is_empty() {
        return Ok(None);
    }
    Ok(Some(OutgoingMessage {
        group_id: String::from_utf8_lossy(group.group_id().as_slice()).into_owned(),
        epoch: group.epoch().as_u64() + 1,
        message_type: MlsMessageType::Commit,
        recipients,
        payload: encode(bundle.commit())?,
        grants: Vec::new(),
    }))
}

/// Lists the identities of all members of the group of a network.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// The identities, ordered, or none, if the gateway is not a member
pub fn members(state: &MlsState, vni: u32) -> Vec<String> {
    let mut members: Vec<String> = state
        .groups
        .get(&vni)
        .map(|group| {
            group
                .members()
                .filter_map(|member| identity_of(&member.credential))
                .collect()
        })
        .unwrap_or_default();
    members.sort();
    members
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use ainari_api_structs::mls_structs::SigningKey;

    /// Signing-key of hanami within the tests
    pub(crate) fn grant_key() -> SigningKey {
        SigningKey::from_bytes(&[5u8; 32])
    }

    /// Signs a grant for a gateway to join the group of a tenant.
    pub(crate) fn grant_for(state: &MlsState, vni: u32) -> MlsGrant {
        let identity = state.identity.as_ref().unwrap();
        MlsGrant::sign(
            &MlsGrantPayload {
                vni,
                client_id: identity.client_id.clone(),
                signature_key: BASE64.encode(identity.signer.public()),
                action: MlsGrantAction::Add,
                issued_at: 0,
                expires_at: i64::MAX,
            },
            &grant_key(),
        )
    }

    /// Creates a gateway with an identity, which is only held in memory.
    pub(crate) fn new_gateway(client_id: &str) -> MlsState {
        let mut state = MlsState::new_in_memory();
        state.create_identity(client_id.to_string()).unwrap();
        state
    }

    /// Prepares the addition of `invited` by `inviter` with a fresh key-package and the grant of
    /// `grant_owner`.
    fn addition_with_grant(
        inviter: &MlsState,
        invited: &MlsState,
        grant_owner: &MlsState,
        vni: u32,
    ) -> Addition {
        let client_id = invited.client_id().unwrap().to_string();
        let encoded = create_key_packages(invited, 1).unwrap().remove(0);
        Addition {
            key_package: decode_key_package(inviter, &client_id, &encoded).unwrap(),
            client_id,
            grant: grant_for(grant_owner, vni),
        }
    }

    /// Prepares the addition of `invited` by `inviter` with a fresh key-package and its grant.
    pub(crate) fn addition(inviter: &MlsState, invited: &MlsState, vni: u32) -> Addition {
        addition_with_grant(inviter, invited, invited, vni)
    }

    /// Lets `inviter` invite `invited` into the group of the tenant and delivers the messages to
    /// `invited` and to all other `members`.
    pub(crate) fn invite(
        inviter: &mut MlsState,
        invited: &mut MlsState,
        members: &mut [&mut MlsState],
        vni: u32,
    ) {
        let addition = addition(inviter, invited, vni);
        let messages = stage_change(inviter, vni, vec![addition], &[]).unwrap();
        merge_pending(inviter, vni).unwrap();
        deliver(&messages, invited, members);
    }

    /// Delivers messages to the gateways, which are named as their recipients.
    pub(crate) fn deliver(
        messages: &[OutgoingMessage],
        first: &mut MlsState,
        others: &mut [&mut MlsState],
    ) {
        for message in messages {
            for recipient in &message.recipients {
                let gateway: &mut MlsState = if first.client_id() == Some(recipient.as_str()) {
                    first
                } else {
                    others
                        .iter_mut()
                        .find(|it| it.client_id() == Some(recipient.as_str()))
                        .expect("unknown recipient")
                };
                process_message(
                    gateway,
                    message.message_type,
                    &message.payload,
                    &message.grants,
                    &grant_key().verifying_key(),
                )
                .unwrap();
            }
        }
    }

    fn secret(state: &MlsState, vni: u32) -> Vec<u8> {
        state.groups[&vni]
            .export_secret(state.provider.crypto(), "test", b"", 32)
            .unwrap()
    }

    #[test]
    fn invited_gateways_share_the_secret_of_the_group() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");

        assert!(!create_group(&mut a, 5).unwrap());
        invite(&mut a, &mut b, &mut [], 5);
        assert_eq!(secret(&a, 5), secret(&b, 5));

        // any member can invite the next gateway, the others follow with the commit
        invite(&mut b, &mut c, &mut [&mut a], 5);
        assert_eq!(secret(&a, 5), secret(&c, 5));
        assert_eq!(secret(&b, 5), secret(&c, 5));
        assert_eq!(a.groups[&5].epoch().as_u64(), 2);
        assert_eq!(members(&c, 5), vec!["10.0.0.1", "10.0.0.2", "10.0.0.3"]);
    }

    #[test]
    fn a_removed_gateway_loses_the_secret() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        invite(&mut a, &mut c, &mut [&mut b], 5);
        let old_secret = secret(&c, 5);

        let messages = stage_change(&mut a, 5, Vec::new(), &["10.0.0.3".to_string()]).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].recipients, vec!["10.0.0.2"]);
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut []);

        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_ne!(secret(&a, 5), old_secret);
        assert_eq!(members(&a, 5), vec!["10.0.0.1", "10.0.0.2"]);
        assert!(!is_member(&a, 5, "10.0.0.3"));
    }

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
    fn a_gateway_can_be_invited_again_after_it_lost_its_group() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        // b forgot the group, for example because its last VM of the network was deleted
        delete_group(&mut b, 5).unwrap();
        assert!(is_member(&a, 5, "10.0.0.2"));

        // the old leaf goes with the commit, which adds the new one
        invite(&mut a, &mut b, &mut [], 5);

        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_eq!(members(&a, 5).len(), 2);
        assert_eq!(a.groups[&5].members().count(), 2);
    }

    #[test]
    fn a_re_add_is_one_commit() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        invite(&mut a, &mut c, &mut [&mut b], 5);
        let epoch = a.groups[&5].epoch().as_u64();

        delete_group(&mut c, 5).unwrap();
        let addition = addition(&a, &c, 5);
        let messages = stage_change(&mut a, 5, vec![addition], &[]).unwrap();
        // the commit only goes to the remaining member, the re-added gateway gets the welcome
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_type, MlsMessageType::Commit);
        assert_eq!(messages[0].recipients, vec!["10.0.0.2"]);
        assert_eq!(messages[1].message_type, MlsMessageType::Welcome);
        assert_eq!(messages[1].recipients, vec!["10.0.0.3"]);
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut [&mut c]);

        assert_eq!(a.groups[&5].epoch().as_u64(), epoch + 1);
        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_eq!(secret(&a, 5), secret(&c, 5));
        assert_eq!(a.groups[&5].members().count(), 3);
    }

    #[test]
    fn several_gateways_are_added_with_one_commit() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        let mut d = new_gateway("10.0.0.4");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        let epoch = a.groups[&5].epoch().as_u64();

        let additions = vec![addition(&a, &c, 5), addition(&a, &d, 5)];
        let messages = stage_change(&mut a, 5, additions, &[]).unwrap();
        assert_eq!(messages[0].grants.len(), 2);
        assert_eq!(messages[1].recipients, vec!["10.0.0.3", "10.0.0.4"]);
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut [&mut c, &mut d]);

        assert_eq!(a.groups[&5].epoch().as_u64(), epoch + 1);
        for gateway in [&b, &c, &d] {
            assert_eq!(secret(&a, 5), secret(gateway, 5));
        }
        assert_eq!(
            members(&d, 5),
            vec!["10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.4"]
        );
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

    #[tokio::test]
    async fn a_gateway_without_key_package_does_not_stop_the_others() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        let d = new_gateway("10.0.0.4");
        let mut e = new_gateway("10.0.0.5");
        let f = new_gateway("10.0.0.6");
        let impostor = new_gateway("10.0.0.6");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        // d has no key-package left, the one of f was made by an impostor
        let mut key_packages = std::collections::HashMap::new();
        for gateway in [&c, &e, &impostor] {
            let client_id = gateway.client_id().unwrap().to_string();
            key_packages.insert(
                client_id,
                create_key_packages(gateway, 1).unwrap().remove(0),
            );
        }
        let adds: Vec<MlsOperationAdd> = [&c, &d, &e, &f]
            .into_iter()
            .map(|gateway| MlsOperationAdd {
                client_id: gateway.client_id().unwrap().to_string(),
                grant: grant_for(gateway, 5),
            })
            .collect();
        let claim = |target: String, _signature_key: String| {
            let key_package = key_packages.get(&target).cloned();
            async move { key_package.ok_or_else(|| "no key-package left".to_string()) }
        };
        let (additions, refused) =
            check_additions(&a, 5, &adds, &grant_key().verifying_key(), 0, claim).await;

        let refused: Vec<&str> = refused.iter().map(|it| it.client_id.as_str()).collect();
        assert_eq!(refused, vec!["10.0.0.4", "10.0.0.6"]);
        let messages = stage_change(&mut a, 5, additions, &[]).unwrap();
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut [&mut c, &mut e]);

        for gateway in [&b, &c, &e] {
            assert_eq!(secret(&a, 5), secret(gateway, 5));
        }
        assert_eq!(
            members(&a, 5),
            vec!["10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.5"]
        );
    }

    #[tokio::test]
    async fn an_expired_grant_is_refused_before_a_key_package_is_claimed() {
        let a = new_gateway("10.0.0.1");
        let c = new_gateway("10.0.0.3");
        let mut payload = grant_for(&c, 5)
            .verify(&grant_key().verifying_key())
            .unwrap();
        payload.expires_at = 100;
        let adds = vec![MlsOperationAdd {
            client_id: "10.0.0.3".to_string(),
            grant: MlsGrant::sign(&payload, &grant_key()),
        }];

        let claim = |_: String, _: String| async { panic!("no key-package is claimed") };
        let (additions, refused) =
            check_additions(&a, 5, &adds, &grant_key().verifying_key(), 101, claim).await;
        assert!(additions.is_empty());
        assert_eq!(refused[0].reason, "the grant is expired");
    }

    #[test]
    fn only_a_removal_for_good_or_a_self_update_moves_all_pairs() {
        let change = |added: &[&str], removed: &[&str]| CommitChange {
            committer: None,
            added: added.iter().map(|it| it.to_string()).collect(),
            removed: removed.iter().map(|it| it.to_string()).collect(),
        };
        assert!(change(&[], &[]).moves_all());
        assert!(change(&[], &["c"]).moves_all());
        assert!(change(&["d"], &["c"]).moves_all());
        assert!(!change(&["c"], &[]).moves_all());
        // a re-add only moves the pairs of the re-added gateway
        assert!(!change(&["c"], &["c"]).moves_all());
    }

    #[test]
    fn a_change_can_neither_be_empty_nor_remove_the_committer() {
        let mut a = new_gateway("10.0.0.1");
        create_group(&mut a, 5).unwrap();

        assert!(matches!(
            stage_change(&mut a, 5, Vec::new(), &[]),
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(matches!(
            stage_change(&mut a, 5, Vec::new(), &["10.0.0.1".to_string()]),
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(matches!(
            stage_change(&mut a, 5, Vec::new(), &["10.0.0.9".to_string()]),
            Err(ErrorResponse::NotFound(_))
        ));
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

    #[test]
    fn a_key_package_of_another_gateway_is_refused() {
        let a = new_gateway("10.0.0.1");
        let b = new_gateway("10.0.0.2");
        let encoded = create_key_packages(&b, 1).unwrap().remove(0);

        assert!(decode_key_package(&a, "10.0.0.2", &encoded).is_ok());
        assert!(matches!(
            decode_key_package(&a, "10.0.0.3", &encoded),
            Err(ErrorResponse::BadRequest(_))
        ));
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
    fn a_grant_only_covers_its_gateway_and_tenant() {
        let c = new_gateway("10.0.0.3");
        let grant = grant_for(&c, 5);
        let key = grant_key().verifying_key();

        assert!(verify_grant(&grant, &key, 5, "10.0.0.3").is_ok());
        assert!(verify_grant(&grant, &key, 6, "10.0.0.3").is_err());
        assert!(verify_grant(&grant, &key, 5, "10.0.0.4").is_err());
    }

    #[test]
    fn a_self_update_rotates_the_secret_of_all_members() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        let old_secret = secret(&a, 5);

        let commit = stage_update(&mut a, 5).unwrap().unwrap();
        merge_pending(&mut a, 5).unwrap();
        deliver(&[commit], &mut b, &mut []);

        assert_ne!(secret(&a, 5), old_secret);
        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_eq!(members(&a, 5), vec!["10.0.0.1", "10.0.0.2"]);
    }
}
