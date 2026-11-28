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

//! Coordination-state of the groups: their members and their committer, and which of the
//! gateways are alive.

use crate::database::mls_client_table;
use crate::database::mls_group_table::{self, MlsGroupEntry};
use crate::database::mls_operation_table;

use super::internal_error;

use ainari_api::errors::ErrorResponse;

/// Prefix of the id of the group of a network, which the gateways use as well
pub(crate) const GROUP_ID_PREFIX: &str = "ainari-vni-";

/// Seconds without contact, after which a gateway counts as dead
pub const LIVENESS_TIMEOUT: i64 = 45;

/// Coordination-state of the group of a network
#[derive(Debug, Clone)]
pub(crate) struct Group {
    pub(crate) vni: u32,
    pub(crate) committer: String,
    pub(crate) epoch: u64,
    pub(crate) members: Vec<String>,
    pub(crate) rotated_at: i64,
    pub(crate) created_at: i64,
}

impl Group {
    pub(crate) fn from_entry(entry: MlsGroupEntry) -> Self {
        Group {
            vni: entry.vni,
            committer: entry.committer,
            epoch: entry.epoch.max(0) as u64,
            members: parse_list(Some(&entry.members)),
            rotated_at: entry.rotated_at,
            created_at: entry.created_at,
        }
    }

    pub(crate) fn to_entry(&self) -> MlsGroupEntry {
        MlsGroupEntry {
            vni: self.vni,
            committer: self.committer.clone(),
            epoch: self.epoch as i64,
            members: serde_json::to_string(&self.members).unwrap_or_else(|_| "[]".to_string()),
            rotated_at: self.rotated_at,
            created_at: self.created_at,
        }
    }

    /// Records the epoch and the members of the group after a commit.
    pub(crate) fn advance(&mut self, epoch: u64, members: &[String], now: i64) {
        let mut members = members.to_vec();
        members.sort();
        members.dedup();
        self.epoch = epoch;
        self.members = members;
        self.rotated_at = now;
    }

    pub(crate) fn is_member(&self, client_id: &str) -> bool {
        self.members.iter().any(|member| member == client_id)
    }
}

/// Reads a JSON-encoded list of identities.
pub(crate) fn parse_list(value: Option<&str>) -> Vec<String> {
    value
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_default()
}

pub(crate) fn load_group(vni: u32) -> Result<Option<Group>, ErrorResponse> {
    mls_group_table::get_group(vni)
        .map(|entry| entry.map(Group::from_entry))
        .map_err(|e| internal_error(&format!("read the MLS-group of tenant {vni}"), e))
}

pub(crate) fn save_group(group: &Group) -> Result<(), ErrorResponse> {
    mls_group_table::save_group(group.to_entry())
        .map_err(|e| internal_error(&format!("store the MLS-group of tenant {}", group.vni), e))
}

/// Records a contact of a gateway, which keeps it alive.
pub(crate) fn touch(client_id: &str, now: i64) -> Result<(), ErrorResponse> {
    mls_client_table::touch_client(client_id, now)
        .map_err(|e| internal_error("record the contact of a gateway", e))
}

/// Checks, if a gateway was in touch within the liveness-timeout.
pub(crate) fn is_alive(client_id: &str, now: i64) -> Result<bool, ErrorResponse> {
    let last_seen = mls_client_table::last_seen_of(client_id)
        .map_err(|e| internal_error("read the last contact of a gateway", e))?;
    Ok(last_seen.is_some_and(|last_seen| now - last_seen < LIVENESS_TIMEOUT))
}

/// Unix-time in seconds, since which the izakaya runs. Before the liveness-timeout passed, a
/// gateway, which didn't contact the izakaya yet, can't be told apart from a gone one.
pub(crate) static RUNNING_SINCE: std::sync::OnceLock<i64> = std::sync::OnceLock::new();

/// Checks, if another member of a group than the given gateway is alive.
///
/// Without the start of the izakaya, for example in the tests, the izakaya counts as running for
/// long.
pub(crate) fn others_alive(
    group: &Group,
    client_id: &str,
    now: i64,
) -> Result<bool, ErrorResponse> {
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

/// Picks a member of a group, which is alive, as new committer.
///
/// # Arguments
/// * `group` - The group
/// * `exclude` - Members, which must not become committer
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// The new committer, or `None` if no other member is alive
pub(crate) fn pick_committer(
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
pub(crate) fn drop_lost_members(
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
