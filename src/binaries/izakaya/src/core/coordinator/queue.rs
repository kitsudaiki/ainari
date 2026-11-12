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

//! Queue of the changes of the groups. Every change is one row of `mls_operations`, the rows of a
//! batch go to the committer together.

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::database::mls_grant_table::get_grant;
use crate::database::mls_operation_table::{
    self, MlsOperationEntry, STATUS_DONE, STATUS_INFLIGHT, STATUS_QUEUED,
};

use super::group::Group;
use super::internal_error;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;

/// Number of times an operation is handed to a committer again, before it is given up
pub const MAX_OPERATION_RETRIES: i32 = 3;

/// Kind of a queued change of a group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MlsOperationKind {
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

pub(crate) fn open_operations(vni: u32) -> Result<Vec<MlsOperationEntry>, ErrorResponse> {
    mls_operation_table::list_open_operations(vni)
        .map_err(|e| internal_error(&format!("read the operations of tenant {vni}"), e))
}

pub(crate) fn update_operation(op: &MlsOperationEntry) -> Result<(), ErrorResponse> {
    mls_operation_table::update_operation(op)
        .map_err(|e| internal_error(&format!("store the operation '{}'", op.uuid), e))
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
pub(crate) fn enqueue(
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
pub(crate) fn drop_queued_additions(vni: u32, client_id: &str) -> Result<(), ErrorResponse> {
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

/// Lists the gateways, which operations add or remove.
///
/// # Arguments
/// * `ops` - The operations
///
/// # Returns
/// The identities of the gateways, ordered and without duplicates
pub(crate) fn targets<'a>(ops: impl IntoIterator<Item = &'a MlsOperationEntry>) -> Vec<String> {
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
pub(crate) fn is_pointless(
    group: &Group,
    op: &MlsOperationEntry,
    now: i64,
) -> Result<bool, ErrorResponse> {
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
/// Returns the operations of the batch, which is in flight.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `batch_uuid` - Id of the batch
///
/// # Returns
/// The operations, or `Conflict` if the batch is not in flight
pub(crate) fn batch_in_flight(
    vni: u32,
    batch_uuid: Uuid,
) -> Result<Vec<MlsOperationEntry>, ErrorResponse> {
    let batch: Vec<MlsOperationEntry> = open_operations(vni)?
        .into_iter()
        .filter(|op| op.batch_uuid == Some(batch_uuid) && op.status == STATUS_INFLIGHT)
        .collect();
    if batch.is_empty() {
        return Err(ErrorResponse::Conflict(format!(
            "Operation '{batch_uuid}' is not in flight"
        )));
    }
    Ok(batch)
}

/// Finishes all operations of a batch, after the committer reported its result. Only an
/// addition can be left out by the committer, which is queued again.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `batch` - The operations of the batch
/// * `refused` - The gateways, which the committer left out
///
/// # Returns
/// `Ok(())` once all operations are updated
pub(crate) fn finish_batch(
    vni: u32,
    batch: &mut [MlsOperationEntry],
    refused: &[MlsOperationRefusal],
) -> Result<(), ErrorResponse> {
    let add = MlsOperationKind::Add.to_string();
    for op in batch.iter_mut() {
        let refusal = refused
            .iter()
            .find(|refusal| op.kind == add && op.client_id.as_ref() == Some(&refusal.client_id));
        finish_operation(vni, op, refusal)?;
    }
    Ok(())
}

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
