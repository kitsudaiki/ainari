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

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;

use crate::database::quota_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::quota_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "quota",
    summary = "Set quota",
    description = r###"Update the quota of a specific user.

Only the values, which are not 0, are applied, so single limits can be changed
without providing all of them. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn set_quota_admin(
    quota_id: Path<String>,
    body: Json<QuotaSetReq>,
    context: UserContext,
) -> Result<Json<QuotaResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;

    // get current quota of user from database
    let mut current_quota = quota_table::get_quota(&quota_id, &context)
        .map_err(|e| map_db_id_get_delete_error("quota", &quota_id, e))?;

    // update values to set
    if body.max_virtual_machine != 0 {
        current_quota.max_virtual_machine = body.max_virtual_machine;
    }
    if body.max_image != 0 {
        current_quota.max_image = body.max_image;
    }
    if body.max_secret != 0 {
        current_quota.max_secret = body.max_secret;
    }
    if body.max_network != 0 {
        current_quota.max_network = body.max_network;
    }
    if body.max_floating_ip != 0 {
        current_quota.max_floating_ip = body.max_floating_ip;
    }

    // update values in database
    quota_table::set_quota(
        &quota_id,
        current_quota.max_virtual_machine,
        current_quota.max_image,
        current_quota.max_secret,
        current_quota.max_network,
        current_quota.max_floating_ip,
        &context,
    )
    .map_err(|e| map_db_id_get_delete_error("quota", &quota_id, e))?;

    // get new quota of user from database
    let quota = quota_table::get_quota(&quota_id, &context)
        .map_err(|e| map_db_id_get_delete_error("quota", &quota_id, e))?;

    let resp = QuotaResp {
        user_id: quota.id,
        max_virtual_machine: quota.max_virtual_machine,
        max_image: quota.max_image,
        max_secret: quota.max_secret,
        max_network: quota.max_network,
        max_floating_ip: quota.max_floating_ip,
        created_by: quota.created_by,
        created_at: quota.created_at,
        updated_by: quota.updated_by,
        updated_at: quota.updated_at,
    };

    Ok(Json(resp))
}
