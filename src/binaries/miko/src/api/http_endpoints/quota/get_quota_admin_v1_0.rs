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
    description = r###"Get information of the quota of a specific user from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_quota_admin(
    user_id: Path<String>,
    context: UserContext,
) -> Result<Json<QuotaResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;

    // get quota of user from database
    let quota = quota_table::get_quota(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("quota", &user_id, e))?;

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
