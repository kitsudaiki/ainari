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

use actix_web::web::{Json, Path};
use apistos::actix::NoContent;
use apistos::api_operation;
use validator::Validate;

use crate::core::coordinator::{
    self, batch_in_flight, drop_lost_members, exclusively, finish_batch, load_group, progress,
    save_group, targets, touch,
};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_group",
    summary = "Finish MLS-operation",
    description = r###"Report the result of a change of a MLS-group, which the committer made.

A successful change moves the group into its new epoch, and the next batch of changes is handed
to the committer. The gateways roll out the keys of the new epoch themselves.
A refused change, for example because the grant of the gateway, which should be added, is not
valid, is dropped."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn finish_mls_operation_internal(
    path: Path<MlsGroupVniPath>,
    body: Json<MlsOperationDoneReq>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    operation_done(path.vni, &body, coordinator::now())?;

    Ok(NoContent)
}

/// Takes the result of a batch of operations from the committer and hands the next batch out.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `req` - The result of the operation
/// * `now` - Current unix-time in seconds
///
/// # Returns
/// `Ok(())`, `NotFound` if the group is gone or `Conflict` if the operation is not in flight or
/// the sender is not the committer
pub(crate) fn operation_done(
    vni: u32,
    req: &MlsOperationDoneReq,
    now: i64,
) -> Result<(), ErrorResponse> {
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
        let mut batch = batch_in_flight(vni, req.op_uuid)?;
        finish_batch(vni, &mut batch, &req.refused)?;
        let targets = targets(&batch);

        if req.success {
            group.advance(req.epoch, &req.members, now);
            save_group(&group)?;
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
