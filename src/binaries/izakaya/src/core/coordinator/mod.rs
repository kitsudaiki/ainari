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
//!   the *committer* of the group, the only member, which commits. One batch of changes at a
//!   time, so the members never create concurrent commits.
//!
//! The keys of a new epoch are rolled out by the gateways themselves, every pair of gateways on
//! its own, so izakaya doesn't wait for the members after a change.
//!
//! izakaya doesn't decide, who may join: a gateway is only added with a membership-grant of
//! hanami, which the committer and all other members verify again. A compromised izakaya can stall
//! the groups, but it can't bring a gateway into a group.
//!
//! All state lives in the database. Every change of the state runs exclusively - within this
//! process by a mutex, across all instances by a lock of the database - so any number of izakaya
//! instances can serve the gateways.
//!
//! - `group`: the coordination-state of a group, its committer and the liveness of the gateways
//! - `queue`: the queued changes of the groups
//! - `batch`: hands the queued changes to the committers, a batch at a time
//!
//! The changes, which a single request makes (subscribe, unsubscribe, store a grant, report the
//! result of a batch), live in their endpoints. This module keeps the background-ticker, which
//! moves the groups on with the time.

mod batch;
mod group;
mod queue;
#[cfg(test)]
mod tests;

use std::sync::atomic::Ordering;
use std::sync::{Mutex, PoisonError};

use chrono::Utc;

use crate::database::db_handle::{self, DB_CONN};
use crate::database::mls_operation_table;
use crate::database::{mls_client_table, mls_group_table};

pub(crate) use batch::*;
pub(crate) use group::*;
pub(crate) use queue::*;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::ProjectRole;

/// Sender of the messages, which izakaya creates itself
const SENDER: &str = "izakaya";

/// Serializes all changes of the coordination-state within this process
static COORDINATOR: Mutex<()> = Mutex::new(());

/// Logs an error and turns it into the generic `InternalError`.
pub(crate) fn internal_error(action: &str, e: impl std::fmt::Display) -> ErrorResponse {
    ainari_api::report_internal_error!(None, "Failed to {action}: {e}");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Current unix-time in seconds
pub(crate) fn now_secs() -> i64 {
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
pub(crate) fn exclusively<T>(
    function: impl FnOnce() -> Result<T, ErrorResponse>,
) -> Result<T, ErrorResponse> {
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

/// Starts a group of a network again with one gateway as its only member.
pub(crate) fn reset_group(vni: u32, client_id: &str, now: i64) -> Result<(), ErrorResponse> {
    mls_operation_table::drop_open_operations(vni)
        .map_err(|e| internal_error(&format!("drop the operations of tenant {vni}"), e))?;
    save_group(&Group {
        vni,
        committer: client_id.to_string(),
        epoch: 0,
        members: vec![client_id.to_string()],
        rotated_at: now,
        created_at: now,
    })?;
    log::info!("'{client_id}' creates the MLS-group of tenant {vni}");
    Ok(())
}

/// Removes a gateway from the group of a network, or drops the whole group, if it was the only
/// member.
pub(crate) fn remove_member(vni: u32, client_id: &str, now: i64) -> Result<(), ErrorResponse> {
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

/// Moves all groups on, which is needed for everything, which only depends on the time: batches,
/// which are due or stall, and the regular key-rotation.
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
/// As long as it is member, its pairs with the other members can't move to new keys. A gateway,
/// which was only cut off for a while, is not lost: it is added again, as soon as it subscribes
/// with its grant again.
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
            for (member, silent_for) in dead_members(&group, now, member_timeout)? {
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

/// Lists the members of a group, which are silent for longer than the timeout. A group without
/// any living member is left alone, because nobody could make the removal.
///
/// # Arguments
/// * `group` - The group
/// * `now` - Current unix-time in seconds
/// * `member_timeout` - Seconds without contact, after which a member is removed
///
/// # Returns
/// The silent members with the seconds since their last contact
fn dead_members(
    group: &Group,
    now: i64,
    member_timeout: i64,
) -> Result<Vec<(String, i64)>, ErrorResponse> {
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
        dead.clear();
    }
    Ok(dead)
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
