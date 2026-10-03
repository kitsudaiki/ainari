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

use crate::core::coordinator;

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

    let action = coordinator::subscribe(path.vni, &body, coordinator::now())?;

    Ok(Json(MlsSubscribeResp { action }))
}
