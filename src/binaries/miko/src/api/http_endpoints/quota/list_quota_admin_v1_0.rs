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

#[api_operation(
    tag = "quota",
    summary = "List quota",
    description = r###"List basic information of all quota from the database. This can only be done by an admin."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_quota_admin(context: UserContext) -> Result<Json<QuotaListResp>, ErrorResponse> {
    if !context.is_admin {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }

    let quotas = match quota_table::list_quotas(&context) {
        Ok(result) => result,
        Err(e) => {
            let msg = format!("Failed to list quotas with error: '{e}'");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let mut resp = QuotaListResp { quotas: Vec::new() };

    for quota in quotas {
        let obj = QuotaBasicResp {
            user_id: quota.id.clone(),
            max_cluster: quota.max_cluster,
            max_dataset: quota.max_dataset,
            max_checkpoint: quota.max_checkpoint,
            max_secret: quota.max_secret,
            max_taskqueue: quota.max_taskqueue,
        };

        resp.quotas.push(obj); // fill the vector with objects
    }

    Ok(Json(resp))
}
