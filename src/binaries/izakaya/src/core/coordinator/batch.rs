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

//! Hands the queued changes of a group to its committer, a batch at a time.

use std::sync::atomic::{AtomicI64, Ordering};

use uuid::Uuid;

use crate::database::mls_message_table;
use crate::database::mls_operation_table::{MlsOperationEntry, STATUS_DONE, STATUS_INFLIGHT};

use super::group::*;
use super::queue::*;
use super::{SENDER, internal_error, system_context};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;

/// Seconds, after which an operation, which doesn't move on, is handed out again
pub const STALL_TIMEOUT: i64 = 30;

/// Default of the milliseconds, which a queued addition or removal waits for others
pub const DEFAULT_COMMIT_DELAY: i64 = 1000;

/// Milliseconds, which a queued addition or removal waits for others, so they are all made with
/// one commit. Set by `spawn_ticker`. The tests hand out every change right away, unless they
/// test the batching.
pub(crate) static COMMIT_DELAY: AtomicI64 =
    AtomicI64::new(if cfg!(test) { 0 } else { DEFAULT_COMMIT_DELAY });

/// Moves the coordination of a group on: it hands a stalled batch out again and the next batch to
/// the committer.
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
pub(crate) fn progress(vni: u32, now: i64) -> Result<(), ErrorResponse> {
    let Some(mut group) = load_group(vni)? else {
        return Ok(());
    };
    let (inflight, queued): (Vec<_>, Vec<_>) = open_operations(vni)?
        .into_iter()
        .partition(|op| op.status == STATUS_INFLIGHT);
    if !inflight.is_empty() {
        return retry_stalled(&mut group, inflight, now);
    }

    let open = drop_pointless(&group, queued, now)?;
    let commit_delay = COMMIT_DELAY.load(Ordering::Relaxed);
    let Some(mut batch) = select_batch(open, now * 1000, commit_delay) else {
        return Ok(());
    };
    if !assign_committer(&mut group, &targets(&batch), now)? {
        return Ok(());
    }
    hand_out(&group, &mut batch, Uuid::new_v4(), now)
}

/// Waits for the batch in flight, until it stalls, and hands it out again afterwards. An
/// operation, which ran out of retries, is given up, and a dead committer is replaced.
///
/// # Arguments
/// * `group` - The group
/// * `inflight` - The operations of the batch in flight
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())` once the batch is waited for or handed out again
fn retry_stalled(
    group: &mut Group,
    mut inflight: Vec<MlsOperationEntry>,
    now: i64,
) -> Result<(), ErrorResponse> {
    let vni = group.vni;
    let started_at = inflight
        .iter()
        .filter_map(|op| op.started_at)
        .min()
        .unwrap_or_default();
    if now * 1000 - started_at <= STALL_TIMEOUT * 1000 {
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
        && let Some(committer) = pick_committer(group, &targets(&batch), now)?
    {
        log::warn!(
            "Committer '{}' of tenant {vni} is dead, '{committer}' takes over",
            group.committer
        );
        group.committer = committer;
        save_group(group)?;
    }
    for op in &mut batch {
        op.started_at = Some(now * 1000);
        update_operation(op)?;
    }
    send_operation(group, &batch)
}

/// Drops the queued operations, which became pointless in the queue.
///
/// # Arguments
/// * `group` - The group
/// * `queued` - The queued operations
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// The operations, which are still to do
fn drop_pointless(
    group: &Group,
    queued: Vec<MlsOperationEntry>,
    now: i64,
) -> Result<Vec<MlsOperationEntry>, ErrorResponse> {
    let mut open = Vec::new();
    for mut op in queued {
        if is_pointless(group, &op, now)? {
            op.status = STATUS_DONE.to_string();
            update_operation(&op)?;
        } else {
            open.push(op);
        }
    }
    Ok(open)
}

/// Selects the next batch from the queued operations of a group.
///
/// All additions and removals go together, once the oldest one waited long enough for the
/// others. An update is never combined with additions, because the members tell a key-rotation
/// by a commit, which adds and removes nobody. Next to a removal, which renews all keys as well,
/// an update is done with the batch.
///
/// # Arguments
/// * `open` - The queued operations, in the order they were queued
/// * `now_ms` - Current unix-time in milliseconds
/// * `commit_delay` - Milliseconds, which an addition or removal waits for others
///
/// # Returns
/// The batch, or `None` if nothing is due
fn select_batch(
    open: Vec<MlsOperationEntry>,
    now_ms: i64,
    commit_delay: i64,
) -> Option<Vec<MlsOperationEntry>> {
    let update = MlsOperationKind::Update.to_string();
    let remove = MlsOperationKind::Remove.to_string();
    let (changes, updates): (Vec<_>, Vec<_>) = open.into_iter().partition(|op| op.kind != update);
    let due = changes
        .iter()
        .map(|op| op.created_at)
        .min()
        .is_some_and(|oldest| now_ms - oldest >= commit_delay);

    if due {
        let removes_any = changes.iter().any(|op| op.kind == remove);
        let mut batch = changes;
        if removes_any {
            batch.extend(updates);
        }
        return Some(batch);
    }
    updates.into_iter().next().map(|update| vec![update])
}

/// Makes sure, that the committer of a group can make a batch: it has to be alive, and it neither
/// removes itself, nor adds itself again after it lost the group.
///
/// # Arguments
/// * `group` - The group
/// * `targets` - The gateways, which the batch adds or removes
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `true` if the group has a committer for the batch, `false` if nobody, who could make it, is
/// alive right now
fn assign_committer(
    group: &mut Group,
    targets: &[String],
    now: i64,
) -> Result<bool, ErrorResponse> {
    if !targets.contains(&group.committer) && is_alive(&group.committer, now)? {
        return Ok(true);
    }
    match pick_committer(group, targets, now)? {
        Some(committer) => {
            group.committer = committer;
            save_group(group)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Puts a batch in flight and hands it to the committer.
///
/// # Arguments
/// * `group` - The group
/// * `batch` - The operations of the batch
/// * `batch_uuid` - Id of the batch, which the committer reports the result with
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())` once the batch is sent
fn hand_out(
    group: &Group,
    batch: &mut [MlsOperationEntry],
    batch_uuid: Uuid,
    now: i64,
) -> Result<(), ErrorResponse> {
    for op in batch.iter_mut() {
        op.status = STATUS_INFLIGHT.to_string();
        op.started_at = Some(now * 1000);
        op.batch_uuid = Some(batch_uuid);
        update_operation(op)?;
    }
    send_operation(group, batch)
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
pub(crate) fn send_operation(
    group: &Group,
    batch: &[MlsOperationEntry],
) -> Result<(), ErrorResponse> {
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

/// Hands a message to members of a group.
pub(crate) fn send(
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
