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

use crate::core::coordinator::{self, exclusively, remove_member, touch};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_group",
    summary = "Unsubscribe from MLS-group",
    description = r###"Unsubscribe a gateway from the MLS-group of a network, because it serves no VM of it anymore.

The gateway is removed from the group by the committer, which rotates the keys of the remaining
members. The last member takes the group with it."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn unsubscribe_mls_group_internal(
    path: Path<MlsGroupVniPath>,
    body: Json<MlsUnsubscribeReq>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    unsubscribe(path.vni, &body.client_id, coordinator::now())?;

    Ok(NoContent)
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
pub(crate) fn unsubscribe(vni: u32, client_id: &str, now: i64) -> Result<(), ErrorResponse> {
    exclusively(|| {
        touch(client_id, now)?;
        remove_member(vni, client_id, now)
    })
}
