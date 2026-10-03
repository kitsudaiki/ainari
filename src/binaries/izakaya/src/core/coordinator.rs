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

//! Coordination of the MLS-groups of the gateways.
//!
//! The gateways run themselves: each one subscribes to the group of every network, which has a
//! VM behind it, and unsubscribes, when the last one is gone. izakaya turns that into an ordered
//! sequence of changes per group:
//!
//! - **Operations**: every change of a group (add, remove, key-rotation) is queued and handed to
//!   the *committer* of the group, the only member, which commits. One change at a time, so the
//!   members never create concurrent commits.
//! - **Rounds**: after every change the keys of the new epoch are rolled out without packet loss
//!   in three phases. Each phase only starts, once every member acknowledged the previous one:
//!   *install* the incoming keys of the new epoch, *switch* the outgoing traffic to them and,
//!   after a short grace period for the packets on their way, *cleanup* the old incoming keys.
//!
//! izakaya doesn't decide, who may join: a gateway is only added with a membership-grant of
//! hanami, which the committer and all other members verify again. A compromised izakaya can stall
//! the groups, but it can't bring a gateway into a group.
//!
//! All state lives in the database. Every change of the state runs exclusively - within this
//! process by a mutex, across all instances by a lock of the database - so any number of izakaya
//! instances can serve the gateways.

use std::sync::{Mutex, PoisonError};

use chrono::Utc;
use uuid::Uuid;

use crate::database::db_handle::DB_CONN;
use crate::database::mls_grant_table::{self, MlsGrantEntry};
use crate::database::mls_group_table::{self, MlsGroupEntry};
use crate::database::mls_message_table;
use crate::database::mls_operation_table::{
    self, MlsOperationEntry, STATUS_DONE, STATUS_INFLIGHT, STATUS_QUEUED,
};
use crate::database::{mls_client_table, mls_grant_table::get_grant};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::ProjectRole;

/// Prefix of the id of the group of a network, which the gateways use as well
const GROUP_ID_PREFIX: &str = "ainari-vni-";

/// Sender of the messages, which izakaya creates itself
const SENDER: &str = "izakaya";

/// Seconds, after which a round or an operation, which doesn't move on, is given up
pub const STALL_TIMEOUT: i64 = 30;

/// Seconds without contact, after which a gateway counts as dead
pub const LIVENESS_TIMEOUT: i64 = 45;

/// Seconds between the switch of the outgoing traffic of all members and the removal of the old
/// incoming keys, so the packets, which were sent with the old keys, still arrive
pub const CLEANUP_GRACE: i64 = 2;

/// Number of times an operation is handed to a committer again, before it is given up
pub const MAX_OPERATION_RETRIES: i32 = 3;

/// Serializes all changes of the coordination-state within this process
static COORDINATOR: Mutex<()> = Mutex::new(());

/// State of a round within the coordination-state of a group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoundState {
    /// Waiting for all members to install the incoming keys of the new epoch
    Install,
    /// Waiting for all members to switch their outgoing traffic to the new epoch
    Switch,
    /// Waiting for the packets, which were sent with the old keys
    Grace,
}

impl RoundState {
    fn as_str(&self) -> &'static str {
        match self {
            RoundState::Install => "install",
            RoundState::Switch => "switch",
            RoundState::Grace => "grace",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "install" => Some(RoundState::Install),
            "switch" => Some(RoundState::Switch),
            "grace" => Some(RoundState::Grace),
            _ => None,
        }
    }
}

/// Key-rotation of a group, which is in progress
#[derive(Debug, Clone)]
struct Round {
    epoch: u64,
    state: RoundState,
    acks: Vec<String>,
    updated_at: i64,
}

/// Coordination-state of the group of a network
#[derive(Debug, Clone)]
struct Group {
    vni: u32,
    committer: String,
    epoch: u64,
    members: Vec<String>,
    round: Option<Round>,
    rotated_at: i64,
    created_at: i64,
}

impl Group {
    fn from_entry(entry: MlsGroupEntry) -> Self {
        let round = match (entry.round_epoch, entry.round_phase.as_deref()) {
            (Some(epoch), Some(phase)) => RoundState::parse(phase).map(|state| Round {
                epoch: epoch.max(0) as u64,
                state,
                acks: parse_list(entry.round_acks.as_deref()),
                updated_at: entry.round_updated_at.unwrap_or_default(),
            }),
            _ => None,
        };
        Group {
            vni: entry.vni,
            committer: entry.committer,
            epoch: entry.epoch.max(0) as u64,
            members: parse_list(Some(&entry.members)),
            round,
            rotated_at: entry.rotated_at,
            created_at: entry.created_at,
        }
    }

    fn to_entry(&self) -> MlsGroupEntry {
        MlsGroupEntry {
            vni: self.vni,
            committer: self.committer.clone(),
            epoch: self.epoch as i64,
            members: serde_json::to_string(&self.members).unwrap_or_else(|_| "[]".to_string()),
            round_epoch: self.round.as_ref().map(|round| round.epoch as i64),
            round_phase: self
                .round
                .as_ref()
                .map(|round| round.state.as_str().to_string()),
            round_acks: self
                .round
                .as_ref()
                .map(|round| serde_json::to_string(&round.acks).unwrap_or_default()),
            round_updated_at: self.round.as_ref().map(|round| round.updated_at),
            rotated_at: self.rotated_at,
            created_at: self.created_at,
        }
    }

    fn is_member(&self, client_id: &str) -> bool {
        self.members.iter().any(|member| member == client_id)
    }
}

/// Reads a JSON-encoded list of identities.
fn parse_list(value: Option<&str>) -> Vec<String> {
    value
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_default()
}

/// Logs an error and turns it into the generic `InternalError`.
fn internal_error(action: &str, e: impl std::fmt::Display) -> ErrorResponse {
    log::error!("Failed to {action}: {e}");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Current unix-time in seconds
fn now_secs() -> i64 {
    Utc::now().timestamp()
}

/// Runs a change of the coordination-state exclusively, within this process and across all
/// instances of izakaya, which share the database.
fn exclusively<T>(function: impl FnOnce() -> Result<T, ErrorResponse>) -> Result<T, ErrorResponse> {
    let _guard = COORDINATOR.lock().unwrap_or_else(PoisonError::into_inner);
    DB_CONN
        .run_exclusively("mls-coordinator", function)
        .map_err(|e| internal_error("lock the coordination-state", e))?
}

/// Context, with which izakaya writes its own messages
fn system_context() -> UserContext {
    UserContext {
        token: String::new(),
        user_id: SENDER.to_string(),
        project_id: String::new(),
        is_admin: false.to_string(),
        project_role: ProjectRole::Member.to_string(),
    }
}

fn load_group(vni: u32) -> Result<Option<Group>, ErrorResponse> {
    mls_group_table::get_group(vni)
        .map(|entry| entry.map(Group::from_entry))
        .map_err(|e| internal_error(&format!("read the MLS-group of tenant {vni}"), e))
}

fn save_group(group: &Group) -> Result<(), ErrorResponse> {
    mls_group_table::save_group(group.to_entry())
        .map_err(|e| internal_error(&format!("store the MLS-group of tenant {}", group.vni), e))
}

fn open_operations(vni: u32) -> Result<Vec<MlsOperationEntry>, ErrorResponse> {
    mls_operation_table::list_open_operations(vni)
        .map_err(|e| internal_error(&format!("read the operations of tenant {vni}"), e))
}

fn update_operation(op: &MlsOperationEntry) -> Result<(), ErrorResponse> {
    mls_operation_table::update_operation(op)
        .map_err(|e| internal_error(&format!("store the operation '{}'", op.uuid), e))
}

/// Checks, if a gateway was in touch within the liveness-timeout.
fn is_alive(client_id: &str, now: i64) -> Result<bool, ErrorResponse> {
    let last_seen = mls_client_table::last_seen_of(client_id)
        .map_err(|e| internal_error("read the last contact of a gateway", e))?;
    Ok(last_seen.is_some_and(|last_seen| now - last_seen < LIVENESS_TIMEOUT))
}

/// Records a contact of a gateway, which keeps it alive.
fn touch(client_id: &str, now: i64) -> Result<(), ErrorResponse> {
    mls_client_table::touch_client(client_id, now)
        .map_err(|e| internal_error("record the contact of a gateway", e))
}

/// Picks a member of a group, which is alive, as new committer.
///
/// # Arguments
/// * `group` - The group
/// * `exclude` - A member, which must not become committer
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// The new committer, or `None` if no other member is alive
fn pick_committer(
    group: &Group,
    exclude: Option<&str>,
    now: i64,
) -> Result<Option<String>, ErrorResponse> {
    for member in &group.members {
        if Some(member.as_str()) == exclude {
            continue;
        }
        if is_alive(member, now)? {
            return Ok(Some(member.clone()));
        }
    }
    Ok(None)
}

/// Hands a message to members of a group.
fn send(
    group: &Group,
    epoch: u64,
    message_type: MlsMessageType,
    recipients: Vec<String>,
    payload: String,
) -> Result<(), ErrorResponse> {
    if recipients.is_empty() {
        return Ok(());
    }
    let req = MlsMessageReq {
        group_id: format!("{GROUP_ID_PREFIX}{}", group.vni),
        epoch,
        message_type,
        sender: SENDER.to_string(),
        recipients,
        payload,
        grants: Vec::new(),
    };
    mls_message_table::add_mls_message(&req, &system_context())
        .map(|_| ())
        .map_err(|e| internal_error(&format!("send a {message_type}"), e))
}

/// Hands a phase of the current round to all members of a group.
fn send_round(group: &Group, epoch: u64, phase: MlsRoundPhase) -> Result<(), ErrorResponse> {
    let message = MlsRoundMessage {
        vni: group.vni,
        epoch,
        phase,
    };
    let payload =
        serde_json::to_string(&message).map_err(|e| internal_error("encode a round-message", e))?;
    send(
        group,
        epoch,
        MlsMessageType::Round,
        group.members.clone(),
        payload,
    )
}

/// Hands an operation to the committer of a group.
fn send_operation(group: &Group, op: &MlsOperationEntry) -> Result<(), ErrorResponse> {
    let kind = op
        .kind
        .parse::<MlsOperationKind>()
        .map_err(|e| internal_error("read an operation", e))?;
    let grant = op
        .grant_json
        .as_deref()
        .and_then(|grant| serde_json::from_str::<MlsGrant>(grant).ok());
    let message = MlsOperation {
        op_uuid: op.uuid,
        vni: group.vni,
        kind,
        client_id: op.client_id.clone(),
        grant,
    };
    let payload =
        serde_json::to_string(&message).map_err(|e| internal_error("encode an operation", e))?;
    send(
        group,
        group.epoch,
        MlsMessageType::Operation,
        vec![group.committer.clone()],
        payload,
    )
}

/// Queues a change of a group, unless the same change is queued already.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `kind` - Kind of the change
/// * `client_id` - Gateway, which is added or removed
/// * `grant` - Grant of hanami for a gateway, which is added
///
/// # Returns
/// `Ok(())` once the change is queued
fn enqueue(
    vni: u32,
    kind: MlsOperationKind,
    client_id: Option<&str>,
    grant: Option<&MlsGrant>,
) -> Result<(), ErrorResponse> {
    let kind_str = kind.to_string();
    let already_queued = open_operations(vni)?
        .iter()
        .any(|op| op.kind == kind_str && op.client_id.as_deref() == client_id);
    if already_queued {
        return Ok(());
    }

    let grant_json = match grant {
        Some(grant) => {
            Some(serde_json::to_string(grant).map_err(|e| internal_error("encode a grant", e))?)
        }
        None => None,
    };
    mls_operation_table::add_operation(MlsOperationEntry {
        uuid: Uuid::new_v4(),
        vni,
        kind: kind_str,
        client_id: client_id.map(str::to_string),
        grant_json,
        status: STATUS_QUEUED.to_string(),
        retries: 0,
        created_at: Utc::now().timestamp_millis(),
        started_at: None,
    })
    .map_err(|e| internal_error(&format!("queue a {kind} for tenant {vni}"), e))?;

    log::debug!(
        "Queued {kind} of '{}' for the MLS-group of tenant {vni}",
        client_id.unwrap_or("-")
    );
    Ok(())
}

/// Drops the queued additions of a gateway, for example because it doesn't want to join anymore.
fn drop_queued_additions(vni: u32, client_id: &str) -> Result<(), ErrorResponse> {
    let add = MlsOperationKind::Add.to_string();
    for mut op in open_operations(vni)? {
        if op.status == STATUS_QUEUED
            && op.kind == add
            && op.client_id.as_deref() == Some(client_id)
        {
            op.status = STATUS_DONE.to_string();
            update_operation(&op)?;
        }
    }
    Ok(())
}

/// Starts a group of a network again with one gateway as its only member.
fn reset_group(vni: u32, client_id: &str, now: i64) -> Result<(), ErrorResponse> {
    mls_operation_table::drop_open_operations(vni)
        .map_err(|e| internal_error(&format!("drop the operations of tenant {vni}"), e))?;
    save_group(&Group {
        vni,
        committer: client_id.to_string(),
        epoch: 0,
        members: vec![client_id.to_string()],
        round: None,
        rotated_at: now,
        created_at: now,
    })?;
    log::info!("'{client_id}' creates the MLS-group of tenant {vni}");
    Ok(())
}

/// Removes a gateway from the group of a network, or drops the whole group, if it was the only
/// member.
fn remove_member(vni: u32, client_id: &str, now: i64) -> Result<(), ErrorResponse> {
    drop_queued_additions(vni, client_id)?;

    let Some(group) = load_group(vni)? else {
        return Ok(());
    };
    if !group.is_member(client_id) {
        return Ok(());
    }
    if group.members.len() == 1 {
        mls_operation_table::drop_open_operations(vni)
            .map_err(|e| internal_error(&format!("drop the operations of tenant {vni}"), e))?;
        mls_group_table::delete_group(vni)
            .map_err(|e| internal_error(&format!("delete the MLS-group of tenant {vni}"), e))?;
        log::info!("The MLS-group of tenant {vni} is gone with its last member '{client_id}'");
        return Ok(());
    }

    enqueue(vni, MlsOperationKind::Remove, Some(client_id), None)?;
    progress(vni, now)
}

/// Moves the coordination of a group on: it advances the current round, gives up a stalled
/// round or operation and hands the next operation to the committer.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())` once the group moved on as far as it can for now
fn progress(vni: u32, now: i64) -> Result<(), ErrorResponse> {
    let Some(mut group) = load_group(vni)? else {
        return Ok(());
    };

    // the round of the last change has to finish, before the next change is made
    if let Some(round) = group.round.clone() {
        let complete = group
            .members
            .iter()
            .all(|member| round.acks.contains(member));
        match round.state {
            RoundState::Install | RoundState::Switch if complete => {
                let next = match round.state {
                    RoundState::Install => {
                        send_round(&group, round.epoch, MlsRoundPhase::Switch)?;
                        RoundState::Switch
                    }
                    _ => RoundState::Grace,
                };
                group.round = Some(Round {
                    epoch: round.epoch,
                    state: next,
                    acks: Vec::new(),
                    updated_at: now,
                });
                save_group(&group)?;
                return Ok(());
            }
            RoundState::Install | RoundState::Switch => {
                if now - round.updated_at <= STALL_TIMEOUT {
                    return Ok(());
                }
                // the members, which moved on, still hold the keys of both epochs, so a stalled
                // round leaves every pair in sync. The next round cleans up.
                log::warn!(
                    "Round of epoch {} of tenant {vni} stalled in phase {}, missing: {:?}",
                    round.epoch,
                    round.state.as_str(),
                    group
                        .members
                        .iter()
                        .filter(|member| !round.acks.contains(member))
                        .collect::<Vec<_>>()
                );
                group.round = None;
                save_group(&group)?;
            }
            RoundState::Grace => {
                if now - round.updated_at < CLEANUP_GRACE {
                    return Ok(());
                }
                send_round(&group, round.epoch, MlsRoundPhase::Cleanup)?;
                group.round = None;
                save_group(&group)?;
                log::debug!("Rotated the keys of tenant {vni} to epoch {}", round.epoch);
            }
        }
    }

    let now_ms = now * 1000;
    let mut ops = open_operations(vni)?;

    // an operation in flight is waited for, until it stalls
    if let Some(op) = ops.iter_mut().find(|op| op.status == STATUS_INFLIGHT) {
        if now_ms - op.started_at.unwrap_or_default() <= STALL_TIMEOUT * 1000 {
            return Ok(());
        }
        op.retries += 1;
        if op.retries > MAX_OPERATION_RETRIES {
            log::error!(
                "Give up the {} of '{}' for tenant {vni} after {} retries",
                op.kind,
                op.client_id.as_deref().unwrap_or("-"),
                MAX_OPERATION_RETRIES
            );
            op.status = STATUS_DONE.to_string();
            update_operation(op)?;
            return progress(vni, now);
        }
        if !is_alive(&group.committer, now)?
            && let Some(committer) = pick_committer(&group, op.client_id.as_deref(), now)?
        {
            log::warn!(
                "Committer '{}' of tenant {vni} is dead, '{committer}' takes over",
                group.committer
            );
            group.committer = committer;
            save_group(&group)?;
        }
        op.started_at = Some(now_ms);
        update_operation(op)?;
        return send_operation(&group, op);
    }

    let Some(op) = ops.iter_mut().find(|op| op.status == STATUS_QUEUED) else {
        return Ok(());
    };

    // an operation, which became pointless in the queue, is dropped
    let kind = op.kind.parse::<MlsOperationKind>().ok();
    let target = op.client_id.clone().unwrap_or_default();
    let pointless = match kind {
        Some(MlsOperationKind::Add) => get_grant(vni, &target)
            .map_err(|e| internal_error("read a grant", e))?
            .is_none_or(|grant| grant.expires_at < now),
        Some(MlsOperationKind::Remove) => !group.is_member(&target),
        Some(MlsOperationKind::Update) => group.members.len() < 2,
        None => true,
    };
    if pointless {
        op.status = STATUS_DONE.to_string();
        update_operation(op)?;
        return progress(vni, now);
    }

    // the committer neither removes itself, nor adds itself again after it lost the group
    let committer_is_target = group.committer == target;
    if committer_is_target || !is_alive(&group.committer, now)? {
        let exclude = committer_is_target.then_some(target.as_str());
        match pick_committer(&group, exclude, now)? {
            Some(committer) => {
                group.committer = committer;
                save_group(&group)?;
            }
            // nobody, who could make the change, is alive right now
            None => return Ok(()),
        }
    }

    op.status = STATUS_INFLIGHT.to_string();
    op.started_at = Some(now_ms);
    update_operation(op)?;
    send_operation(&group, op)
}

/// Stores a grant of hanami.
///
/// A grant, which adds a gateway, allows it to join the group of the network. A grant, which
/// removes a gateway, drops its grant and removes it from the group.
///
/// # Arguments
/// * `grant` - The signed grant
/// * `key` - Public key of hanami
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())`, or `BadRequest` if the grant is not valid
pub fn store_grant(grant: &MlsGrant, key: &VerifyingKey, now: i64) -> Result<(), ErrorResponse> {
    let payload = grant
        .verify(key)
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid grant: {e}")))?;
    if payload.is_expired_at(now) {
        return Err(ErrorResponse::BadRequest(
            "Invalid grant: it is expired".to_string(),
        ));
    }

    exclusively(|| match payload.action {
        MlsGrantAction::Add => {
            mls_grant_table::set_grant(MlsGrantEntry {
                uuid: Uuid::new_v4(),
                vni: payload.vni,
                client_id: payload.client_id.clone(),
                signature_key: payload.signature_key.clone(),
                payload: grant.payload.clone(),
                signature: grant.signature.clone(),
                expires_at: payload.expires_at,
                created_at: Utc::now(),
            })
            .map_err(|e| internal_error("store a grant", e))?;
            // hanami refreshes the grants regularly, so this is no news
            log::debug!(
                "'{}' may join the MLS-group of tenant {}",
                payload.client_id,
                payload.vni
            );
            Ok(())
        }
        MlsGrantAction::Remove => {
            mls_grant_table::delete_grant(payload.vni, &payload.client_id)
                .map_err(|e| internal_error("delete a grant", e))?;
            log::info!(
                "'{}' must leave the MLS-group of tenant {}",
                payload.client_id,
                payload.vni
            );
            remove_member(payload.vni, &payload.client_id, now)
        }
    })
}

/// Subscribes a gateway to the group of a network.
///
/// The gateway needs a grant of hanami. If the network has no group yet, the gateway creates it
/// and becomes its committer, otherwise it is added by the committer. A member, which lost the
/// group, for example with its database, is added again.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `req` - The subscription
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// What the gateway has to do, or `Forbidden` if hanami didn't allow it to join
pub fn subscribe(
    vni: u32,
    req: &MlsSubscribeReq,
    now: i64,
) -> Result<MlsSubscribeAction, ErrorResponse> {
    exclusively(|| {
        let client_id = req.client_id.as_str();
        touch(client_id, now)?;

        let grant = get_grant(vni, client_id).map_err(|e| internal_error("read a grant", e))?;
        let Some(grant) = grant else {
            return Err(ErrorResponse::Forbidden(format!(
                "'{client_id}' has no grant for the MLS-group of tenant {vni}"
            )));
        };

        let action = match load_group(vni)? {
            None => {
                reset_group(vni, client_id, now)?;
                MlsSubscribeAction::Create
            }
            Some(group) if group.is_member(client_id) && req.has_group => {
                MlsSubscribeAction::Member
            }
            Some(group) if group.is_member(client_id) && group.members.len() == 1 => {
                reset_group(vni, client_id, now)?;
                MlsSubscribeAction::Create
            }
            Some(_) => {
                if grant.expires_at < now {
                    return Err(ErrorResponse::Forbidden(format!(
                        "The grant of '{client_id}' for the MLS-group of tenant {vni} is expired"
                    )));
                }
                enqueue(
                    vni,
                    MlsOperationKind::Add,
                    Some(client_id),
                    Some(&grant.grant()),
                )?;
                MlsSubscribeAction::Joining
            }
        };

        progress(vni, now)?;
        Ok(action)
    })
}

/// Unsubscribes a gateway from the group of a network, which removes it from the group.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `client_id` - Identity of the gateway
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())` once the removal is queued
pub fn unsubscribe(vni: u32, client_id: &str, now: i64) -> Result<(), ErrorResponse> {
    exclusively(|| {
        touch(client_id, now)?;
        remove_member(vni, client_id, now)
    })
}

/// Takes the result of an operation from the committer and starts the round of the new epoch.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `req` - The result of the operation
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())`, `NotFound` if the group is gone or `Conflict` if the operation is not in flight or
/// the sender is not the committer
pub fn operation_done(vni: u32, req: &MlsOperationDoneReq, now: i64) -> Result<(), ErrorResponse> {
    exclusively(|| {
        touch(&req.client_id, now)?;

        let Some(mut group) = load_group(vni)? else {
            return Err(ErrorResponse::NotFound(format!(
                "No MLS-group for tenant {vni}"
            )));
        };
        if group.committer != req.client_id {
            return Err(ErrorResponse::Conflict(format!(
                "'{}' is not the committer of tenant {vni}",
                req.client_id
            )));
        }
        let mut ops = open_operations(vni)?;
        let Some(op) = ops
            .iter_mut()
            .find(|op| op.uuid == req.op_uuid && op.status == STATUS_INFLIGHT)
        else {
            return Err(ErrorResponse::Conflict(format!(
                "Operation '{}' is not in flight",
                req.op_uuid
            )));
        };
        op.status = STATUS_DONE.to_string();
        update_operation(op)?;

        if req.success {
            let mut members = req.members.clone();
            members.sort();
            members.dedup();
            group.epoch = req.epoch;
            group.members = members;
            group.rotated_at = now;
            group.round = Some(Round {
                epoch: req.epoch,
                state: RoundState::Install,
                acks: Vec::new(),
                updated_at: now,
            });
            save_group(&group)?;
            send_round(&group, req.epoch, MlsRoundPhase::Install)?;
            log::info!(
                "MLS-group of tenant {vni} is in epoch {} with {} member(s)",
                req.epoch,
                group.members.len()
            );
        } else {
            log::warn!(
                "Committer '{}' refused the {} of '{}' for tenant {vni}: {}",
                req.client_id,
                op.kind,
                op.client_id.as_deref().unwrap_or("-"),
                req.reason.as_deref().unwrap_or("-")
            );
        }

        progress(vni, now)
    })
}

/// Takes the acknowledgement of a phase of the current round from a member.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `req` - The acknowledgement
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())`, also for an acknowledgement, which is not needed anymore
pub fn round_ack(vni: u32, req: &MlsRoundAckReq, now: i64) -> Result<(), ErrorResponse> {
    exclusively(|| {
        touch(&req.client_id, now)?;

        let Some(mut group) = load_group(vni)? else {
            return Ok(());
        };
        let expected = match group.round.as_ref().map(|round| round.state) {
            Some(RoundState::Install) => MlsRoundPhase::Install,
            Some(RoundState::Switch) => MlsRoundPhase::Switch,
            _ => return Ok(()),
        };
        let is_member = group.is_member(&req.client_id);
        let Some(round) = group.round.as_mut() else {
            return Ok(());
        };
        if round.epoch != req.epoch || req.phase != expected || !is_member {
            return Ok(());
        }
        if !round.acks.contains(&req.client_id) {
            round.acks.push(req.client_id.clone());
            save_group(&group)?;
        }

        progress(vni, now)
    })
}

/// Records, that a gateway is alive, which happens with every poll of its messages.
///
/// # Arguments
/// * `client_id` - Identity of the gateway
pub fn record_contact(client_id: &str) {
    if let Err(e) = touch(client_id, now_secs()) {
        log::warn!("Failed to record the contact of '{client_id}': {e:?}");
    }
}

/// Moves all groups on, which is needed for everything, which only depends on the time: rounds
/// and operations, which stall, the grace-period of a round and the regular key-rotation.
///
/// # Arguments
/// * `now` - Current unix-time in seconds
/// * `rotation_interval` - Seconds between two key-rotations of a group
///
/// # Returns
/// `Ok(())` once all groups moved on
pub fn tick(now: i64, rotation_interval: i64) -> Result<(), ErrorResponse> {
    exclusively(|| {
        let vnis = mls_group_table::list_group_vnis()
            .map_err(|e| internal_error("list the MLS-groups", e))?;
        for vni in vnis {
            if let Some(mut group) = load_group(vni)?
                && group.members.len() > 1
                && now - group.rotated_at >= rotation_interval
            {
                log::info!("Rotate the keys of tenant {vni}");
                enqueue(vni, MlsOperationKind::Update, None, None)?;
                group.rotated_at = now;
                save_group(&group)?;
            }
            if let Err(e) = progress(vni, now) {
                log::error!("Failed to move the MLS-group of tenant {vni} on: {e:?}");
            }
        }
        Ok(())
    })
}

/// Starts the background-thread, which calls `tick` every second.
///
/// # Arguments
/// * `rotation_interval` - Seconds between two key-rotations of a group
pub fn spawn_ticker(rotation_interval: i64) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            if let Err(e) = tick(now_secs(), rotation_interval) {
                log::error!("Failed to move the MLS-groups on: {e:?}");
            }
        }
    });
}

/// Current unix-time in seconds, as used by the endpoints
pub fn now() -> i64 {
    now_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    use crate::database::mls_message_table::list_mls_messages;

    const KEY_SEED: [u8; 32] = [3u8; 32];

    fn key() -> SigningKey {
        SigningKey::from_bytes(&KEY_SEED)
    }

    /// Every test uses its own tenant, so the tests don't see each others groups
    fn new_vni() -> u32 {
        (Uuid::new_v4().as_u128() % 100_000) as u32 + 1_000
    }

    fn grant(vni: u32, client_id: &str, action: MlsGrantAction, now: i64) -> MlsGrant {
        MlsGrant::sign(
            &MlsGrantPayload {
                vni,
                client_id: client_id.to_string(),
                signature_key: format!("key-of-{client_id}"),
                action,
                issued_at: now,
                expires_at: now + 600,
            },
            &key(),
        )
    }

    fn allow(vni: u32, client_id: &str, now: i64) {
        store_grant(
            &grant(vni, client_id, MlsGrantAction::Add, now),
            &key().verifying_key(),
            now,
        )
        .unwrap();
    }

    fn subscribe_as(vni: u32, client_id: &str, has_group: bool, now: i64) -> MlsSubscribeAction {
        let req = MlsSubscribeReq {
            client_id: client_id.to_string(),
            has_group,
        };
        subscribe(vni, &req, now).unwrap()
    }

    /// Takes all pending messages of a gateway of one type and acknowledges them.
    fn take(client_id: &str, message_type: MlsMessageType) -> Vec<String> {
        let ctx = system_context();
        list_mls_messages(client_id)
            .unwrap()
            .into_iter()
            .filter(|message| message.message_type == message_type.to_string())
            .map(|message| {
                mls_message_table::delete_mls_message(client_id, &message.uuid, &ctx)
                    .ok()
                    .unwrap();
                message.payload
            })
            .collect()
    }

    fn take_operations(client_id: &str) -> Vec<MlsOperation> {
        take(client_id, MlsMessageType::Operation)
            .iter()
            .map(|payload| serde_json::from_str(payload).unwrap())
            .collect()
    }

    fn take_rounds(client_id: &str) -> Vec<MlsRoundMessage> {
        take(client_id, MlsMessageType::Round)
            .iter()
            .map(|payload| serde_json::from_str(payload).unwrap())
            .collect()
    }

    fn ack(vni: u32, client_id: &str, epoch: u64, phase: MlsRoundPhase, now: i64) {
        let req = MlsRoundAckReq {
            client_id: client_id.to_string(),
            epoch,
            phase,
        };
        round_ack(vni, &req, now).unwrap();
    }

    fn done(vni: u32, op: &MlsOperation, committer: &str, epoch: u64, members: &[&str], now: i64) {
        let req = MlsOperationDoneReq {
            op_uuid: op.op_uuid,
            client_id: committer.to_string(),
            success: true,
            epoch,
            members: members.iter().map(|it| it.to_string()).collect(),
            reason: None,
        };
        operation_done(vni, &req, now).unwrap();
    }

    fn client(name: &str) -> String {
        format!("{name}-{}", Uuid::new_v4())
    }

    #[test]
    #[serial]
    fn a_gateway_without_grant_can_not_subscribe() {
        let vni = new_vni();
        let now = now_secs();
        let req = MlsSubscribeReq {
            client_id: client("a"),
            has_group: false,
        };
        assert!(matches!(
            subscribe(vni, &req, now),
            Err(ErrorResponse::Forbidden(_))
        ));
    }

    #[test]
    #[serial]
    fn a_forged_grant_is_refused() {
        let now = now_secs();
        let forged = MlsGrant::sign(
            &MlsGrantPayload {
                vni: 1,
                client_id: "a".to_string(),
                signature_key: "k".to_string(),
                action: MlsGrantAction::Add,
                issued_at: now,
                expires_at: now + 600,
            },
            &SigningKey::from_bytes(&[9u8; 32]),
        );
        assert!(matches!(
            store_grant(&forged, &key().verifying_key(), now),
            Err(ErrorResponse::BadRequest(_))
        ));
    }

    #[test]
    #[serial]
    fn a_join_is_rolled_out_in_three_phases() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        allow(vni, &a, now);
        allow(vni, &b, now);

        // the first gateway creates the group, the second one is added by it
        assert_eq!(
            subscribe_as(vni, &a, false, now),
            MlsSubscribeAction::Create
        );
        assert_eq!(
            subscribe_as(vni, &b, false, now),
            MlsSubscribeAction::Joining
        );
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].kind, MlsOperationKind::Add);
        assert_eq!(ops[0].client_id.as_deref(), Some(b.as_str()));
        assert!(ops[0].grant.is_some());
        assert!(take_operations(&b).is_empty());

        // the committer reports the new epoch, which starts the round
        done(vni, &ops[0], &a, 1, &[&a, &b], now);
        for member in [&a, &b] {
            let rounds = take_rounds(member);
            assert_eq!(rounds.len(), 1);
            assert_eq!(rounds[0].phase, MlsRoundPhase::Install);
            assert_eq!(rounds[0].epoch, 1);
        }

        // the switch only starts, once both installed the new keys
        ack(vni, &a, 1, MlsRoundPhase::Install, now);
        assert!(take_rounds(&a).is_empty());
        ack(vni, &b, 1, MlsRoundPhase::Install, now);
        assert_eq!(take_rounds(&a)[0].phase, MlsRoundPhase::Switch);
        assert_eq!(take_rounds(&b)[0].phase, MlsRoundPhase::Switch);

        // the cleanup only starts, once both switched and the grace-period is over
        ack(vni, &a, 1, MlsRoundPhase::Switch, now);
        ack(vni, &b, 1, MlsRoundPhase::Switch, now);
        tick(now, 3600).unwrap();
        assert!(take_rounds(&a).is_empty());
        tick(now + CLEANUP_GRACE, 3600).unwrap();
        assert_eq!(take_rounds(&a)[0].phase, MlsRoundPhase::Cleanup);
        assert_eq!(take_rounds(&b)[0].phase, MlsRoundPhase::Cleanup);

        // a member, which still holds the group, is just a member
        assert_eq!(subscribe_as(vni, &b, true, now), MlsSubscribeAction::Member);
    }

    #[test]
    #[serial]
    fn the_next_change_waits_for_the_round_of_the_last_one() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        for gateway in [&a, &b, &c] {
            allow(vni, gateway, now);
        }
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        subscribe_as(vni, &c, false, now);

        // only the addition of b is in flight, c waits
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);
        assert!(take_operations(&a).is_empty());

        ack(vni, &a, 1, MlsRoundPhase::Install, now);
        ack(vni, &b, 1, MlsRoundPhase::Install, now);
        ack(vni, &a, 1, MlsRoundPhase::Switch, now);
        ack(vni, &b, 1, MlsRoundPhase::Switch, now);
        assert!(take_operations(&a).is_empty());

        // with the end of the round, the addition of c is handed to the committer
        tick(now + CLEANUP_GRACE, 3600).unwrap();
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].client_id.as_deref(), Some(c.as_str()));
    }

    #[test]
    #[serial]
    fn a_revoked_gateway_is_removed() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        let ops = take_operations(&a);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);
        for member in [&a, &b] {
            ack(vni, member, 1, MlsRoundPhase::Install, now);
        }
        for member in [&a, &b] {
            ack(vni, member, 1, MlsRoundPhase::Switch, now);
        }
        tick(now + CLEANUP_GRACE, 3600).unwrap();

        store_grant(
            &grant(vni, &b, MlsGrantAction::Remove, now),
            &key().verifying_key(),
            now,
        )
        .unwrap();
        let ops = take_operations(&a);
        let removal = ops
            .iter()
            .find(|op| op.kind == MlsOperationKind::Remove)
            .unwrap();
        assert_eq!(removal.client_id.as_deref(), Some(b.as_str()));

        // without grant, b can't join again
        let req = MlsSubscribeReq {
            client_id: b.clone(),
            has_group: false,
        };
        assert!(matches!(
            subscribe(vni, &req, now),
            Err(ErrorResponse::Forbidden(_))
        ));
    }

    #[test]
    #[serial]
    fn a_stalled_round_is_given_up() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        let ops = take_operations(&a);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);
        ack(vni, &a, 1, MlsRoundPhase::Install, now);

        // b never acknowledges, so nobody switches
        tick(now + STALL_TIMEOUT + 1, 3600).unwrap();
        assert!(load_group(vni).unwrap().unwrap().round.is_none());
        assert!(
            !take_rounds(&a)
                .iter()
                .any(|round| round.phase == MlsRoundPhase::Switch)
        );
    }

    #[test]
    #[serial]
    fn the_keys_are_rotated_regularly() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        let ops = take_operations(&a);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);
        for phase in [MlsRoundPhase::Install, MlsRoundPhase::Switch] {
            ack(vni, &a, 1, phase, now);
            ack(vni, &b, 1, phase, now);
        }
        tick(now + CLEANUP_GRACE, 60).unwrap();
        assert!(take_operations(&a).is_empty());

        // both gateways keep polling in the meantime
        touch(&a, now + 60).unwrap();
        touch(&b, now + 60).unwrap();
        tick(now + 60, 60).unwrap();
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].kind, MlsOperationKind::Update);
    }

    #[test]
    #[serial]
    fn the_last_member_takes_the_group_with_it() {
        let vni = new_vni();
        let now = now_secs();
        let a = client("a");
        allow(vni, &a, now);
        subscribe_as(vni, &a, false, now);

        unsubscribe(vni, &a, now).unwrap();
        assert!(load_group(vni).unwrap().is_none());

        // the next gateway starts with a new group
        assert_eq!(subscribe_as(vni, &a, true, now), MlsSubscribeAction::Create);
    }
}
