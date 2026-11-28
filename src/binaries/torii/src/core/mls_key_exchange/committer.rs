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

//! The changes of a group, which the gateway makes as its committer.
//!
//! izakaya hands every batch of changes of a group to one member, the committer. It makes all
//! additions and removals of the batch with one commit and reports the result. A gateway is only
//! added with a valid grant of hanami, which names exactly the identity and the signature-key of
//! its key-package.

use chrono::Utc;

use crate::config::{CONFIG, INTERNAL_API_KEY};
use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::mls_key_exchange::commit;
use crate::core::mls_key_exchange::gateway::apply_keys;
use crate::core::mls_key_exchange::group;
use crate::core::mls_key_exchange::izakaya::deliver_and_merge;
use crate::core::mls_key_exchange::messages::OutgoingMessage;
use crate::core::mls_key_exchange::state::{MLS_STATE_HANDLE, MlsState};

use ainari_api_structs::mls_structs::*;
use ainari_clients::izakaya as izakaya_clients;
use ainari_common::config::Endpoint;

/// Makes a change of a group as its committer and reports the result to the izakaya.
///
/// # Arguments
/// * `grant_key` - Public key of hanami
/// * `izakaya` - Endpoint of the izakaya
/// * `client_id` - Identity of this gateway
/// * `op` - The change
///
/// # Returns
/// `Ok(())` once the result is reported, also if the change was refused
pub async fn handle_operation(
    grant_key: &VerifyingKey,
    izakaya: &Endpoint,
    client_id: &str,
    op: MlsOperation,
) -> Result<(), String> {
    let vni = op.vni;
    let description = describe(&op);
    let mut mls = MLS_STATE_HANDLE.lock().await;

    let mut refused = Vec::new();
    let result = make_change(grant_key, &mut mls, izakaya, &op, &mut refused).await;
    if let Err(e) = mls.save() {
        log::error!("Failed to persist the MLS-state: {e:?}");
    }
    if result.is_ok() {
        let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
        apply_keys(&mls, &mut ebpf_interf, vni);
    }
    let done = done_request(&mls, &op, client_id, &result, refused);
    drop(mls);

    log_result(&description, vni, &done, &result);
    izakaya_clients::finish_mls_operation(
        izakaya,
        &INTERNAL_API_KEY,
        vni,
        &done,
        CONFIG.skip_tls_verification,
    )
    .await
    .map_err(|e| format!("Failed to report the {description}: {e}"))
}

/// Makes the change of an operation: a key-rotation or a change of the members.
///
/// # Arguments
/// * `grant_key` - Public key of hanami
/// * `mls` - The locked MLS-client
/// * `izakaya` - Endpoint of the izakaya
/// * `op` - The change
/// * `refused` - Takes the gateways, which can't be added
///
/// # Returns
/// `Ok(())` once the group is in its new epoch, or the reason, why no commit was made
async fn make_change(
    grant_key: &VerifyingKey,
    mls: &mut MlsState,
    izakaya: &Endpoint,
    op: &MlsOperation,
    refused: &mut Vec<MlsOperationRefusal>,
) -> Result<(), String> {
    let vni = op.vni;
    if !mls.groups.contains_key(&vni) {
        return Err(format!("this gateway has no MLS-group for tenant {vni}"));
    }
    if op.update {
        return rotate_keys(mls, izakaya, op).await;
    }
    change_members(grant_key, mls, izakaya, op, refused).await
}

/// Rotates the keys of a group with a self-update of this gateway.
///
/// # Arguments
/// * `mls` - The locked MLS-client
/// * `izakaya` - Endpoint of the izakaya
/// * `op` - The change, which must neither add nor remove a gateway
///
/// # Returns
/// `Ok(())` once the group is in its new epoch
async fn rotate_keys(
    mls: &mut MlsState,
    izakaya: &Endpoint,
    op: &MlsOperation,
) -> Result<(), String> {
    if !op.adds.is_empty() || !op.removes.is_empty() {
        return Err("a key-rotation can't be combined with other changes".to_string());
    }
    let commit = commit::stage_update(mls, op.vni).map_err(|e| format!("{e:?}"))?;
    let messages: Vec<OutgoingMessage> = commit.into_iter().collect();
    deliver_and_merge(mls, op.vni, izakaya, &messages).await
}

/// Adds and removes the gateways of an operation with one commit.
///
/// A gateway, which is still member from an earlier membership, for example because it lost
/// the group, loses its old leaf with the same commit, which adds its new key-package. A gateway,
/// which can't be added, is left out and the others are added anyway.
///
/// # Arguments
/// * `grant_key` - Public key of hanami
/// * `mls` - The locked MLS-client
/// * `izakaya` - Endpoint of the izakaya
/// * `op` - The change
/// * `refused` - Takes the gateways, which can't be added
///
/// # Returns
/// `Ok(())` once the group is in its new epoch, or the reason, why no commit was made
async fn change_members(
    grant_key: &VerifyingKey,
    mls: &mut MlsState,
    izakaya: &Endpoint,
    op: &MlsOperation,
    refused: &mut Vec<MlsOperationRefusal>,
) -> Result<(), String> {
    let vni = op.vni;
    let claim = |target: String, signature_key: String| async move {
        izakaya_clients::claim_key_package(
            izakaya,
            &INTERNAL_API_KEY,
            &target,
            Some(signature_key),
            CONFIG.skip_tls_verification,
        )
        .await
        .map(|claimed| claimed.key_package)
        .map_err(|e| format!("no key-package of '{target}': {e}"))
    };
    let (additions, left_out) =
        commit::check_additions(mls, vni, &op.adds, grant_key, Utc::now().timestamp(), claim).await;
    *refused = left_out;

    for addition in &additions {
        if group::is_member(mls, vni, &addition.client_id) {
            log::warn!(
                "'{}' is still member of the MLS-group of tenant {vni}, re-add it",
                addition.client_id
            );
        }
    }
    // a gateway, which already left the group, needs no removal
    let removals: Vec<String> = op
        .removes
        .iter()
        .filter(|target| group::is_member(mls, vni, target))
        .cloned()
        .collect();
    if additions.is_empty() && removals.is_empty() {
        return Err("nothing is left to change".to_string());
    }

    let messages =
        commit::stage_change(mls, vni, additions, &removals).map_err(|e| format!("{e:?}"))?;
    deliver_and_merge(mls, vni, izakaya, &messages).await
}

/// Builds the report of an operation for the izakaya.
///
/// # Arguments
/// * `mls` - The MLS-client after the change
/// * `op` - The change
/// * `client_id` - Identity of this gateway
/// * `result` - The result of the change
/// * `refused` - The gateways, which were left out
///
/// # Returns
/// The report
fn done_request(
    mls: &MlsState,
    op: &MlsOperation,
    client_id: &str,
    result: &Result<(), String>,
    refused: Vec<MlsOperationRefusal>,
) -> MlsOperationDoneReq {
    MlsOperationDoneReq {
        op_uuid: op.op_uuid,
        client_id: client_id.to_string(),
        success: result.is_ok(),
        epoch: mls
            .groups
            .get(&op.vni)
            .map(|group| group.epoch().as_u64())
            .unwrap_or_default(),
        members: group::members(mls, op.vni),
        reason: result.as_ref().err().cloned(),
        refused,
    }
}

/// Logs the result of an operation.
fn log_result(
    description: &str,
    vni: u32,
    done: &MlsOperationDoneReq,
    result: &Result<(), String>,
) {
    match result {
        Ok(()) => log::info!(
            "Made the {description} for tenant {vni} in epoch {}",
            done.epoch
        ),
        Err(e) => log::warn!("Refused the {description} for tenant {vni}: {e}"),
    }
    for refusal in &done.refused {
        log::warn!(
            "Left '{}' out of the {description} for tenant {vni}: {}",
            refusal.client_id,
            refusal.reason
        );
    }
}

/// Describes an operation for the log.
///
/// # Arguments
/// * `op` - The operation
///
/// # Returns
/// For example "change (add: [10.0.0.2], remove: [])" or "key-rotation"
fn describe(op: &MlsOperation) -> String {
    if op.update {
        return "key-rotation".to_string();
    }
    let adds: Vec<&str> = op.adds.iter().map(|add| add.client_id.as_str()).collect();
    format!("change (add: {adds:?}, remove: {:?})", op.removes)
}
