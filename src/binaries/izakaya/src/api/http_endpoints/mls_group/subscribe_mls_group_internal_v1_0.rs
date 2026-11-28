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
use apistos::api_operation;
use validator::Validate;

use crate::core::coordinator::{
    self, MlsOperationKind, enqueue, exclusively, internal_error, load_group, others_alive,
    progress, reset_group, touch,
};
use crate::database::mls_grant_table::get_grant;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_group",
    summary = "Subscribe to MLS-group",
    description = r###"Subscribe a gateway to the MLS-group of a network, because it serves a VM of it.

The gateway needs a membership-grant of hanami. If the network has no group yet, the gateway
creates it and becomes its committer. Otherwise the committer adds it and the gateway gets a
welcome. A gateway, which is member already, but lost the group, is added again."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 500
)]
pub async fn subscribe_mls_group_internal(
    path: Path<MlsGroupVniPath>,
    body: Json<MlsSubscribeReq>,
    _context: UserContext,
) -> Result<Json<MlsSubscribeResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let action = subscribe(path.vni, &body, coordinator::now())?;

    Ok(Json(MlsSubscribeResp { action }))
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
pub(crate) fn subscribe(
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
