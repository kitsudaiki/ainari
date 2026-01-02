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
    summary = "Get quota",
    description = r###"Get information of a quota from the database."###,
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
    if body.max_cluster != 0 {
        current_quota.max_cluster = body.max_cluster;
    }
    if body.max_dataset != 0 {
        current_quota.max_dataset = body.max_dataset;
    }
    if body.max_checkpoint != 0 {
        current_quota.max_checkpoint = body.max_checkpoint;
    }
    if body.max_secret != 0 {
        current_quota.max_secret = body.max_secret;
    }
    if body.max_taskqueue != 0 {
        current_quota.max_taskqueue = body.max_taskqueue;
    }

    // update values in database
    quota_table::set_quota(
        &quota_id,
        current_quota.max_cluster,
        current_quota.max_dataset,
        current_quota.max_checkpoint,
        current_quota.max_secret,
        current_quota.max_taskqueue,
        &context,
    )
    .map_err(|e| map_db_id_get_delete_error("quota", &quota_id, e))?;

    // get new quota of user from database
    let quota = quota_table::get_quota(&quota_id, &context)
        .map_err(|e| map_db_id_get_delete_error("quota", &quota_id, e))?;

    let resp = QuotaResp {
        user_id: quota.id,
        max_cluster: quota.max_cluster,
        max_dataset: quota.max_dataset,
        max_checkpoint: quota.max_checkpoint,
        max_secret: quota.max_secret,
        max_taskqueue: quota.max_taskqueue,
        created_by: quota.created_by,
        created_at: quota.created_at,
        updated_by: quota.updated_by,
        updated_at: quota.updated_at,
    };

    Ok(Json(resp))
}
