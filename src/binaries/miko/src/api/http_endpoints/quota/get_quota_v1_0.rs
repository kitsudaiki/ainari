// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
use apistos::api_operation;

use crate::database::quota_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::quota_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "quota",
    summary = "Get quota",
    description = r###"Get information of a quota from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_quota(context: UserContext) -> Result<Json<QuotaResp>, ErrorResponse> {
    let user_id = &context.user_id;
    match quota_table::get_quota(user_id, &context) {
        Ok(quota) => {
            let resp = QuotaResp {
                user_id: quota.id.clone(),
                max_cluster: quota.max_cluster,
                max_dataset: quota.max_dataset,
                max_checkpoint: quota.max_checkpoint,
                max_secret: quota.max_secret,
                max_taskqueue: quota.max_taskqueue,
                created_by: quota.created_by.clone(),
                created_at: quota.created_at.clone(),
                updated_by: quota.updated_by.clone(),
                updated_at: quota.updated_at.clone(),
            };

            return Ok(Json(resp));
        }
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Quota of user with ID '{user_id}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };
}
