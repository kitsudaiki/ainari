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

use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, PoisonError};

use chrono::Utc;
use uuid::Uuid;

use crate::database::db_handle::{self, DB_CONN};
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

/// Default of the milliseconds, which a queued addition or removal waits for others
pub const DEFAULT_COMMIT_DELAY: i64 = 1000;

/// Milliseconds, which a queued addition or removal waits for others, so they are all made with
/// one commit. Set by `spawn_ticker`. The tests hand out every change right away, unless they
/// test the batching.
static COMMIT_DELAY: AtomicI64 = AtomicI64::new(if cfg!(test) { 0 } else { DEFAULT_COMMIT_DELAY });

/// Kind of a queued change of a group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MlsOperationKind {
    /// Add a gateway, or add it again, if it lost the group
    Add,
    /// Remove a gateway
    Remove,
    /// Rotate the keys with a self-update of the committer
    Update,
}

impl fmt::Display for MlsOperationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MlsOperationKind::Add => "add",
            MlsOperationKind::Remove => "remove",
            MlsOperationKind::Update => "update",
        };
        write!(f, "{s}")
    }
}

impl FromStr for MlsOperationKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(MlsOperationKind::Add),
            "remove" => Ok(MlsOperationKind::Remove),
            "update" => Ok(MlsOperationKind::Update),
            other => Err(format!("Unknown operation '{other}'")),
        }
    }
}

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
    ainari_api::report_internal_error!(None, "Failed to {action}: {e}");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Current unix-time in seconds
fn now_secs() -> i64 {
    // all instances of izakaya compare the times, which the others wrote, so they use the clock
    // of the database-server, which they share
    match db_handle::database_time() {
        Ok(now) => now,
        Err(e) => {
            log::warn!("Failed to read the time of the database-server, use the own one: {e}");
            Utc::now().timestamp()
        }
    }
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

/// Unix-time in seconds, since which the izakaya runs. Before the liveness-timeout passed, a
/// gateway, which didn't contact the izakaya yet, can't be told apart from a gone one.
static RUNNING_SINCE: std::sync::OnceLock<i64> = std::sync::OnceLock::new();

/// Checks, if another member of a group than the given gateway is alive.
///
/// Without the start of the izakaya, for example in the tests, the izakaya counts as running for
/// long.
fn others_alive(group: &Group, client_id: &str, now: i64) -> Result<bool, ErrorResponse> {
    let running_since = RUNNING_SINCE.get().copied().unwrap_or(0);
    if now - running_since < LIVENESS_TIMEOUT {
        return Ok(true);
    }
    for member in &group.members {
        if member != client_id && is_alive(member, now)? {
            return Ok(true);
        }
    }
    Ok(false)
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
/// * `exclude` - Members, which must not become committer
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// The new committer, or `None` if no other member is alive
fn pick_committer(
    group: &Group,
    exclude: &[String],
    now: i64,
) -> Result<Option<String>, ErrorResponse> {
    for member in &group.members {
        if exclude.contains(member) {
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

/// Hands a batch of operations to the committer of a group, which makes all of them with one
/// commit.
///
/// # Arguments
/// * `group` - The group
/// * `batch` - The operations, which share their `batch_uuid`
///
/// # Returns
/// `Ok(())` once the batch is sent
fn send_operation(group: &Group, batch: &[MlsOperationEntry]) -> Result<(), ErrorResponse> {
    let op_uuid = batch
        .first()
        .and_then(|op| op.batch_uuid)
        .ok_or_else(|| internal_error("read an operation", "the batch has no uuid"))?;
    let mut message = MlsOperation {
        op_uuid,
        vni: group.vni,
        adds: Vec::new(),
        removes: Vec::new(),
        update: false,
    };
    let mut update = false;
    for op in batch {
        let kind = op
            .kind
            .parse::<MlsOperationKind>()
            .map_err(|e| internal_error("read an operation", e))?;
        let target = op.client_id.clone().unwrap_or_default();
        match kind {
            MlsOperationKind::Add => {
                let grant = op
                    .grant_json
                    .as_deref()
                    .and_then(|grant| serde_json::from_str::<MlsGrant>(grant).ok())
                    .ok_or_else(|| {
                        internal_error("read an operation", "the addition has no grant")
                    })?;
                message.adds.push(MlsOperationAdd {
                    client_id: target,
                    grant,
                });
            }
            MlsOperationKind::Remove => message.removes.push(target),
            MlsOperationKind::Update => update = true,
        }
    }
    // an update next to a removal is covered by the removal, which renews all keys as well
    message.update = update && message.adds.is_empty() && message.removes.is_empty();

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

/// Lists the gateways, which operations add or remove.
///
/// # Arguments
/// * `ops` - The operations
///
/// # Returns
/// The identities of the gateways, ordered and without duplicates
fn targets<'a>(ops: impl IntoIterator<Item = &'a MlsOperationEntry>) -> Vec<String> {
    let mut targets: Vec<String> = ops
        .into_iter()
        .filter_map(|op| op.client_id.clone())
        .collect();
    targets.sort();
    targets.dedup();
    targets
}

/// Checks, if a queued operation became pointless, so it can be dropped.
///
/// # Arguments
/// * `group` - The group
/// * `op` - The operation
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `true` if the operation has nothing to do anymore
fn is_pointless(group: &Group, op: &MlsOperationEntry, now: i64) -> Result<bool, ErrorResponse> {
    let target = op.client_id.clone().unwrap_or_default();
    Ok(match op.kind.parse::<MlsOperationKind>().ok() {
        Some(MlsOperationKind::Add) => get_grant(group.vni, &target)
            .map_err(|e| internal_error("read a grant", e))?
            .is_none_or(|grant| grant.expires_at < now),
        Some(MlsOperationKind::Remove) => !group.is_member(&target),
        Some(MlsOperationKind::Update) => group.members.len() < 2,
        None => true,
    })
}

/// Queues a change of a group, unless the same change is queued already.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `kind` - Kind of the change
/// * `client_id` - Gateway, which is added or removed
/// * `grant` - Grant of hanami for a gateway, which is added
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())` once the change is queued
fn enqueue(
    vni: u32,
    kind: MlsOperationKind,
    client_id: Option<&str>,
    grant: Option<&MlsGrant>,
    now: i64,
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
        created_at: now * 1000,
        started_at: None,
        batch_uuid: None,
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

    enqueue(vni, MlsOperationKind::Remove, Some(client_id), None, now)?;
    progress(vni, now)
}

/// Moves the coordination of a group on: it advances the current round, gives up a stalled
/// round or batch and hands the next batch to the committer.
///
/// A batch contains all queued additions and removals, once the oldest of them waited for the
/// commit-delay, so the gateways, which subscribe at about the same time, are added with one
/// commit. A key-rotation is handed out alone, while no addition or removal is due.
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
    let (mut inflight, queued): (Vec<_>, Vec<_>) = open_operations(vni)?
        .into_iter()
        .partition(|op| op.status == STATUS_INFLIGHT);

    // the batch in flight is waited for, until it stalls
    if !inflight.is_empty() {
        let started_at = inflight
            .iter()
            .filter_map(|op| op.started_at)
            .min()
            .unwrap_or_default();
        if now_ms - started_at <= STALL_TIMEOUT * 1000 {
            return Ok(());
        }
        for op in &mut inflight {
            op.retries += 1;
        }
        let (given_up, mut batch): (Vec<_>, Vec<_>) = inflight
            .into_iter()
            .partition(|op| op.retries > MAX_OPERATION_RETRIES);
        for mut op in given_up {
            log::error!(
                "Give up the {} of '{}' for tenant {vni} after {MAX_OPERATION_RETRIES} retries",
                op.kind,
                op.client_id.as_deref().unwrap_or("-"),
            );
            op.status = STATUS_DONE.to_string();
            update_operation(&op)?;
        }
        if batch.is_empty() {
            return progress(vni, now);
        }
        if !is_alive(&group.committer, now)?
            && let Some(committer) = pick_committer(&group, &targets(&batch), now)?
        {
            log::warn!(
                "Committer '{}' of tenant {vni} is dead, '{committer}' takes over",
                group.committer
            );
            group.committer = committer;
            save_group(&group)?;
        }
        for op in &mut batch {
            op.started_at = Some(now_ms);
            update_operation(op)?;
        }
        return send_operation(&group, &batch);
    }

    // operations, which became pointless in the queue, are dropped
    let mut open = Vec::new();
    for mut op in queued {
        if is_pointless(&group, &op, now)? {
            op.status = STATUS_DONE.to_string();
            update_operation(&op)?;
        } else {
            open.push(op);
        }
    }

    // all additions and removals go together, once the oldest one waited long enough for the
    // others. An update is never combined with additions, because the members tell a key-rotation
    // by a commit, which adds and removes nobody.
    let update = MlsOperationKind::Update.to_string();
    let (changes, updates): (Vec<_>, Vec<_>) = open.into_iter().partition(|op| op.kind != update);
    let commit_delay = COMMIT_DELAY.load(Ordering::Relaxed);
    let due = changes
        .iter()
        .map(|op| op.created_at)
        .min()
        .is_some_and(|oldest| now_ms - oldest >= commit_delay);
    let remove = MlsOperationKind::Remove.to_string();
    let mut batch = if due {
        let removes_any = changes.iter().any(|op| op.kind == remove);
        let mut batch = changes;
        if removes_any {
            batch.extend(updates);
        }
        batch
    } else if let Some(update) = updates.into_iter().next() {
        vec![update]
    } else {
        return Ok(());
    };

    // the committer neither removes itself, nor adds itself again after it lost the group
    let targets = targets(&batch);
    let committer_is_target = targets.contains(&group.committer);
    if committer_is_target || !is_alive(&group.committer, now)? {
        match pick_committer(&group, &targets, now)? {
            Some(committer) => {
                group.committer = committer;
                save_group(&group)?;
            }
            // nobody, who could make the change, is alive right now
            None => return Ok(()),
        }
    }

    let batch_uuid = Uuid::new_v4();
    for op in &mut batch {
        op.status = STATUS_INFLIGHT.to_string();
        op.started_at = Some(now_ms);
        op.batch_uuid = Some(batch_uuid);
        update_operation(op)?;
    }
    send_operation(&group, &batch)
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
            // Nobody, who could add the gateway, is left: all other members are gone, for
            // example because their hosts were restarted in new pods with new identities. The
            // group starts from scratch with the gateway. A member, which only was cut off for a
            // while, joins the new group again with its grant, which replaces its old group.
            Some(group) if !others_alive(&group, client_id, now)? => {
                log::warn!(
                    "No other member of the MLS-group of tenant {vni} is alive, '{client_id}' \
                     starts it again"
                );
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
                    now,
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

/// Takes the result of a batch of operations from the committer and starts the round of the new
/// epoch.
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
        let mut batch: Vec<MlsOperationEntry> = open_operations(vni)?
            .into_iter()
            .filter(|op| op.batch_uuid == Some(req.op_uuid) && op.status == STATUS_INFLIGHT)
            .collect();
        if batch.is_empty() {
            return Err(ErrorResponse::Conflict(format!(
                "Operation '{}' is not in flight",
                req.op_uuid
            )));
        }
        // only an addition can be left out by the committer
        let add = MlsOperationKind::Add.to_string();
        for op in &mut batch {
            let refusal = req.refused.iter().find(|refusal| {
                op.kind == add && op.client_id.as_ref() == Some(&refusal.client_id)
            });
            finish_operation(vni, op, refusal)?;
        }
        let targets = targets(&batch);

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
                "Committer '{}' refused the change of {targets:?} for tenant {vni}: {}",
                req.client_id,
                req.reason.as_deref().unwrap_or("-")
            );
            // a committer without members doesn't hold the group anymore
            if req.members.is_empty()
                && !drop_lost_members(&mut group, &req.client_id, &targets, now)?
            {
                return Ok(());
            }
        }

        progress(vni, now)
    })
}

/// Finishes an operation, which was in flight. An operation, whose gateway the committer left
/// out, is queued again, until it ran out of retries.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `op` - The operation
/// * `refusal` - Why the committer left the gateway of the operation out, if it did
///
/// # Returns
/// `Ok(())` once the operation is updated
fn finish_operation(
    vni: u32,
    op: &mut MlsOperationEntry,
    refusal: Option<&MlsOperationRefusal>,
) -> Result<(), ErrorResponse> {
    op.status = STATUS_DONE.to_string();
    if let Some(refusal) = refusal {
        op.retries += 1;
        if op.retries > MAX_OPERATION_RETRIES {
            log::warn!(
                "Give up the {} of '{}' for tenant {vni} after {MAX_OPERATION_RETRIES} retries: {}",
                op.kind,
                refusal.client_id,
                refusal.reason
            );
        } else {
            log::warn!(
                "The {} of '{}' for tenant {vni} was left out, retry it: {}",
                op.kind,
                refusal.client_id,
                refusal.reason
            );
            op.status = STATUS_QUEUED.to_string();
            op.started_at = None;
        }
    }
    update_operation(op)
}

/// Drops the committer of a refused operation from a group, because it doesn't hold the group
/// anymore, for example because it left the group, while its own removal was still queued.
///
/// The targets of the operations are dropped as well: a removal would only take its membership
/// away, and a gateway, which waits to be added, doesn't hold the group either. Without this,
/// the group would keep members, which can't make any change, and refuse every gateway forever.
/// If nobody is left, the group is deleted, so the next gateway creates it again.
///
/// # Arguments
/// * `group` - The group
/// * `committer` - The committer, which refused the operation
/// * `targets` - The gateways, which the operations were about
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(true)` if the group still exists, `Ok(false)` if it was deleted
fn drop_lost_members(
    group: &mut Group,
    committer: &str,
    targets: &[String],
    now: i64,
) -> Result<bool, ErrorResponse> {
    let vni = group.vni;
    group
        .members
        .retain(|member| member != committer && !targets.contains(member));

    if group.members.is_empty() {
        mls_operation_table::drop_open_operations(vni)
            .map_err(|e| internal_error(&format!("drop the operations of tenant {vni}"), e))?;
        mls_group_table::delete_group(vni)
            .map_err(|e| internal_error(&format!("delete the MLS-group of tenant {vni}"), e))?;
        log::warn!(
            "No member of the MLS-group of tenant {vni} holds it anymore, so it is deleted and \
             created again by the next gateway"
        );
        return Ok(false);
    }

    log::warn!(
        "Committer '{committer}' doesn't hold the MLS-group of tenant {vni} anymore, so it is \
         dropped from it"
    );
    // progress picks another one, if the new committer is not alive
    group.committer = pick_committer(group, &[], now)?.unwrap_or_else(|| group.members[0].clone());
    save_group(group)?;
    Ok(true)
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
                enqueue(vni, MlsOperationKind::Update, None, None, now)?;
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

/// Removes the members of all groups, which didn't contact the izakaya for a while.
///
/// Every gateway polls its messages every second, so a gateway, which stays silent, is gone, for
/// example because its host was restarted in a new pod with a new address and a new identity.
/// As long as it is member, every round waits for it until it stalls, so the group never rotates
/// its keys again. A gateway, which was only cut off for a while, is not lost: it is added again,
/// as soon as it subscribes with its grant again.
///
/// Nothing is removed, as long as the izakaya itself doesn't run longer than the timeout, because
/// the gateways couldn't contact it before, and from groups without any living member, which
/// couldn't make the removal anyway.
///
/// # Arguments
/// * `now` - Current unix-time in seconds
/// * `member_timeout` - Seconds without contact, after which a member is removed
/// * `running_since` - Unix-time in seconds, since which this izakaya runs
///
/// # Returns
/// `Ok(())` once the removals are queued
pub fn remove_dead_members(
    now: i64,
    member_timeout: i64,
    running_since: i64,
) -> Result<(), ErrorResponse> {
    if now - running_since < member_timeout {
        return Ok(());
    }
    exclusively(|| {
        let vnis = mls_group_table::list_group_vnis()
            .map_err(|e| internal_error("list the MLS-groups", e))?;
        for vni in vnis {
            let Some(group) = load_group(vni)? else {
                continue;
            };
            let mut dead = Vec::new();
            let mut any_alive = false;
            for member in &group.members {
                let last_seen = mls_client_table::last_seen_of(member)
                    .map_err(|e| internal_error("read the last contact of a gateway", e))?;
                match last_seen {
                    Some(last_seen) if now - last_seen >= member_timeout => {
                        dead.push((member.clone(), now - last_seen))
                    }
                    Some(_) => any_alive = true,
                    None => {}
                }
            }
            if !any_alive {
                continue;
            }
            for (member, silent_for) in dead {
                log::warn!(
                    "'{member}' was silent for {silent_for} seconds, remove it from the \
                     MLS-group of tenant {vni}"
                );
                remove_member(vni, &member, now)?;
            }
        }
        Ok(())
    })
}

/// Starts the background-thread, which calls `tick` and `remove_dead_members` every second.
///
/// # Arguments
/// * `rotation_interval` - Seconds between two key-rotations of a group
/// * `member_timeout` - Seconds without contact, after which a member is removed
/// * `commit_delay` - Milliseconds, which a queued addition or removal waits for others
pub fn spawn_ticker(rotation_interval: i64, member_timeout: i64, commit_delay: i64) {
    COMMIT_DELAY.store(commit_delay, Ordering::Relaxed);
    let running_since = *RUNNING_SINCE.get_or_init(now_secs);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let now = now_secs();
            if let Err(e) = tick(now, rotation_interval) {
                log::error!("Failed to move the MLS-groups on: {e:?}");
            }
            if let Err(e) = remove_dead_members(now, member_timeout, running_since) {
                log::error!("Failed to remove the silent members of the MLS-groups: {e:?}");
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
            refused: Vec::new(),
        };
        operation_done(vni, &req, now).unwrap();
    }

    /// Lists the gateways, which an operation adds
    fn added(op: &MlsOperation) -> Vec<&str> {
        op.adds.iter().map(|add| add.client_id.as_str()).collect()
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
        assert_eq!(added(&ops[0]), vec![b.as_str()]);
        assert!(ops[0].removes.is_empty());
        assert!(!ops[0].update);
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
        assert_eq!(added(&ops[0]), vec![c.as_str()]);
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
        let removal = ops.iter().find(|op| !op.removes.is_empty()).unwrap();
        assert_eq!(removal.removes, vec![b.clone()]);

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

    /// Reports, that the committer refused an operation, because it has no group
    fn refuse_without_group(vni: u32, op: &MlsOperation, committer: &str, now: i64) {
        let req = MlsOperationDoneReq {
            op_uuid: op.op_uuid,
            client_id: committer.to_string(),
            success: false,
            epoch: 0,
            members: Vec::new(),
            reason: Some(format!("this gateway has no MLS-group for tenant {vni}")),
            refused: Vec::new(),
        };
        operation_done(vni, &req, now).unwrap();
    }

    /// Builds a group of the given gateways, whose first one is the committer
    fn joined_group(vni: u32, gateways: &[&String], now: i64) {
        for gateway in gateways {
            allow(vni, gateway, now);
            subscribe_as(vni, gateway, false, now);
        }
        for epoch in 1..gateways.len() as u64 {
            let ops = take_operations(gateways[0]);
            let members = &gateways[..=epoch as usize];
            let members: Vec<&str> = members.iter().map(|it| it.as_str()).collect();
            done(vni, &ops[0], gateways[0], epoch, &members, now);
            for phase in [MlsRoundPhase::Install, MlsRoundPhase::Switch] {
                for member in &gateways[..=epoch as usize] {
                    ack(vni, member, epoch, phase, now);
                }
            }
            tick(now + CLEANUP_GRACE, 3600).unwrap();
        }
        for gateway in gateways {
            take_rounds(gateway);
        }
    }

    #[test]
    #[serial]
    fn a_group_is_created_again_after_its_last_members_left_at_once() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        joined_group(vni, &[&a, &b], now);

        // both gateways drop the group at the same time, so the committer a refuses the removal
        // of b, because it already left the group itself
        unsubscribe(vni, &b, now).unwrap();
        let ops = take_operations(&a);
        assert_eq!(ops[0].removes, vec![b.clone()]);
        refuse_without_group(vni, &ops[0], &a, now);
        unsubscribe(vni, &a, now).unwrap();
        assert!(load_group(vni).unwrap().is_none());

        // later both need the encryption again: the first creates the group, the second joins
        assert_eq!(
            subscribe_as(vni, &b, false, now),
            MlsSubscribeAction::Create
        );
        assert_eq!(
            subscribe_as(vni, &a, false, now),
            MlsSubscribeAction::Joining
        );
        let ops = take_operations(&b);
        assert_eq!(added(&ops[0]), vec![a.as_str()]);
    }

    #[test]
    #[serial]
    fn a_committer_without_group_is_replaced_by_a_member_with_it() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        joined_group(vni, &[&a, &b, &c], now);

        // a lost the group, so it refuses the removal of b, while c still holds the group
        unsubscribe(vni, &b, now).unwrap();
        let ops = take_operations(&a);
        refuse_without_group(vni, &ops[0], &a, now);

        let group = load_group(vni).unwrap().unwrap();
        assert_eq!(group.members, vec![c.clone()]);
        assert_eq!(group.committer, c);

        // a joins again over c
        assert_eq!(
            subscribe_as(vni, &a, false, now),
            MlsSubscribeAction::Joining
        );
        let ops = take_operations(&c);
        assert_eq!(added(&ops[0]), vec![a.as_str()]);
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
        assert!(ops[0].update);
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

    #[test]
    #[serial]
    fn a_silent_member_is_removed() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        let ops = take_operations(&a);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);
        take_operations(&a);

        // b stays silent, a keeps polling
        let later = now + 200;
        touch(&a, later).unwrap();

        // an izakaya, which only runs since a moment, can't tell, who is silent
        remove_dead_members(later, 120, later - 10).unwrap();
        assert!(!take_operations(&a).iter().any(|op| !op.removes.is_empty()));

        remove_dead_members(later, 120, now).unwrap();
        tick(later + STALL_TIMEOUT + 1, 3600).unwrap();
        let ops = take_operations(&a);
        let removal = ops
            .iter()
            .find(|op| !op.removes.is_empty())
            .expect("b is removed");
        assert_eq!(removal.removes, vec![b.clone()]);
    }

    #[test]
    #[serial]
    fn a_group_without_living_member_is_left_alone() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        let ops = take_operations(&a);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);

        remove_dead_members(now + 200, 120, now - 200).unwrap();
        let group = load_group(vni).unwrap().unwrap();
        assert_eq!(group.members.len(), 2);
        assert!(
            !open_operations(vni)
                .unwrap()
                .iter()
                .any(|op| op.kind == MlsOperationKind::Remove.to_string())
        );
    }

    #[test]
    #[serial]
    fn a_group_without_living_members_starts_again() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        allow(vni, &c, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        let ops = take_operations(&a);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);

        // a and b are gone, while c comes along
        let later = now + LIVENESS_TIMEOUT + 10;
        assert_eq!(
            subscribe_as(vni, &c, false, later),
            MlsSubscribeAction::Create
        );
        let group = load_group(vni).unwrap().unwrap();
        assert_eq!(group.members, vec![c.clone()]);
        assert_eq!(group.committer, c);
        assert!(open_operations(vni).unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn a_gateway_joins_while_a_member_is_alive() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        allow(vni, &c, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);
        let ops = take_operations(&a);
        done(vni, &ops[0], &a, 1, &[&a, &b], now);

        // b is gone, but a still polls
        let later = now + LIVENESS_TIMEOUT + 10;
        touch(&a, later).unwrap();
        assert_eq!(
            subscribe_as(vni, &c, false, later),
            MlsSubscribeAction::Joining
        );
    }

    /// Reports, that the committer left all gateways of an operation out
    fn refuse_gateways(vni: u32, op: &MlsOperation, committer: &str, epoch: u64, now: i64) {
        let req = MlsOperationDoneReq {
            op_uuid: op.op_uuid,
            client_id: committer.to_string(),
            success: false,
            epoch,
            members: vec![committer.to_string()],
            reason: Some("every change was refused".to_string()),
            refused: op
                .adds
                .iter()
                .map(|add| MlsOperationRefusal {
                    client_id: add.client_id.clone(),
                    reason: "no key-package".to_string(),
                })
                .collect(),
        };
        operation_done(vni, &req, now).unwrap();
    }

    #[test]
    #[serial]
    fn a_refused_gateway_is_queued_again_until_it_ran_out_of_retries() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b) = (client("a"), client("b"));
        allow(vni, &a, now);
        allow(vni, &b, now);
        subscribe_as(vni, &a, false, now);
        subscribe_as(vni, &b, false, now);

        for _ in 0..MAX_OPERATION_RETRIES {
            let ops = take_operations(&a);
            assert_eq!(ops.len(), 1);
            assert_eq!(added(&ops[0]), vec![b.as_str()]);
            refuse_gateways(vni, &ops[0], &a, 0, now);
        }
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        refuse_gateways(vni, &ops[0], &a, 0, now);

        // the group is unchanged, and b is only added again with its next subscription
        assert!(take_operations(&a).is_empty());
        assert_eq!(load_group(vni).unwrap().unwrap().members, vec![a.clone()]);
        subscribe_as(vni, &b, false, now);
        assert_eq!(added(&take_operations(&a)[0]), vec![b.as_str()]);
    }

    /// Sets the commit-delay for a test and resets it, when the test ends
    struct CommitDelay;

    impl CommitDelay {
        fn set(delay: i64) -> Self {
            COMMIT_DELAY.store(delay, Ordering::Relaxed);
            CommitDelay
        }
    }

    impl Drop for CommitDelay {
        fn drop(&mut self) {
            COMMIT_DELAY.store(0, Ordering::Relaxed);
        }
    }

    /// Lets `a` create a group and `others` subscribe at the same time, and returns the batch,
    /// which is handed to `a` after the commit-delay
    fn subscribe_together(vni: u32, a: &str, others: &[&String], now: i64) -> MlsOperation {
        allow(vni, a, now);
        subscribe_as(vni, a, false, now);
        for gateway in others {
            allow(vni, gateway, now);
            assert_eq!(
                subscribe_as(vni, gateway, false, now),
                MlsSubscribeAction::Joining
            );
        }
        // the additions wait for each other
        tick(now, 3600).unwrap();
        assert!(take_operations(a).is_empty());

        tick(now + 1, 3600).unwrap();
        let mut ops = take_operations(a);
        assert_eq!(ops.len(), 1);
        ops.remove(0)
    }

    #[test]
    #[serial]
    fn gateways_which_subscribe_together_are_added_with_one_commit() {
        let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c, d) = (client("a"), client("b"), client("c"), client("d"));

        let op = subscribe_together(vni, &a, &[&b, &c, &d], now);
        let mut expected = vec![b.as_str(), c.as_str(), d.as_str()];
        expected.sort();
        let mut adds = added(&op);
        adds.sort();
        assert_eq!(adds, expected);
        assert!(op.removes.is_empty());
        assert!(!op.update);

        // the whole batch is finished with one report
        done(vni, &op, &a, 1, &[&a, &b, &c, &d], now + 1);
        assert!(open_operations(vni).unwrap().is_empty());
        assert_eq!(load_group(vni).unwrap().unwrap().members.len(), 4);
    }

    #[test]
    #[serial]
    fn a_stalled_batch_is_retried_as_a_whole() {
        let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        let op = subscribe_together(vni, &a, &[&b, &c], now);

        tick(now + 1 + STALL_TIMEOUT, 3600).unwrap();
        assert!(take_operations(&a).is_empty());
        tick(now + 2 + STALL_TIMEOUT, 3600).unwrap();
        let retried = take_operations(&a);
        assert_eq!(retried.len(), 1);
        assert_eq!(retried[0].op_uuid, op.op_uuid);
        assert_eq!(retried[0].adds.len(), 2);
    }

    #[test]
    #[serial]
    fn only_the_refused_gateway_of_a_batch_is_queued_again() {
        let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        let op = subscribe_together(vni, &a, &[&b, &c], now);

        let req = MlsOperationDoneReq {
            op_uuid: op.op_uuid,
            client_id: a.clone(),
            success: true,
            epoch: 1,
            members: vec![a.clone(), b.clone()],
            reason: None,
            refused: vec![MlsOperationRefusal {
                client_id: c.clone(),
                reason: "no key-package".to_string(),
            }],
        };
        operation_done(vni, &req, now + 1).unwrap();

        let open = open_operations(vni).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].client_id.as_deref(), Some(c.as_str()));
        assert_eq!(open[0].status, STATUS_QUEUED);
        assert_eq!(open[0].retries, 1);
    }

    #[test]
    #[serial]
    fn a_key_rotation_is_never_combined_with_additions() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        joined_group(vni, &[&a, &b], now);
        let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);

        allow(vni, &c, now);
        subscribe_as(vni, &c, false, now);
        // the rotation is queued, when the addition of c is due
        tick(now + 1, 0).unwrap();
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        assert_eq!(added(&ops[0]), vec![c.as_str()]);
        assert!(!ops[0].update);

        // the rotation waits for its own turn
        let update = MlsOperationKind::Update.to_string();
        let open = open_operations(vni).unwrap();
        assert!(
            open.iter()
                .any(|op| op.kind == update && op.status == STATUS_QUEUED)
        );
    }

    #[test]
    #[serial]
    fn a_key_rotation_is_covered_by_a_removal() {
        let vni = new_vni();
        let now = now_secs();
        let (a, b, c) = (client("a"), client("b"), client("c"));
        joined_group(vni, &[&a, &b, &c], now);
        let _delay = CommitDelay::set(DEFAULT_COMMIT_DELAY);

        unsubscribe(vni, &c, now).unwrap();
        assert!(take_operations(&a).is_empty());
        tick(now + 1, 0).unwrap();
        let ops = take_operations(&a);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].removes, vec![c.clone()]);
        assert!(!ops[0].update);

        // the removal renews all keys, so the rotation is done with it
        done(vni, &ops[0], &a, 3, &[&a, &b], now + 1);
        assert!(open_operations(vni).unwrap().is_empty());
    }

    #[test]
    #[serial]
    fn the_time_comes_from_the_database() {
        let database = db_handle::database_time().unwrap();
        assert!((database - Utc::now().timestamp()).abs() <= 2);
        assert!((now_secs() - database).abs() <= 2);
    }
}
