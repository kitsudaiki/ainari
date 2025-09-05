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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::checkpoint_table;

use super::checkpoint_structs::CheckpointResp;

use ainari_api::errors::ErrorResponse;
use ainari_api::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "checkpoint",
    summary = "Get checkpoint",
    description = r###"Get information of a checkpoint from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_checkpoint(
    checkpoint_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<CheckpointResp>, ErrorResponse> {
    match checkpoint_table::get_checkpoint(&checkpoint_uuid, &context) {
        Ok(checkpoint) => {
            let resp = CheckpointResp {
                uuid: *checkpoint_uuid,
                name: checkpoint.name.clone(),
                created_by: checkpoint.created_by.clone(),
                created_at: checkpoint.created_at.clone(),
                updated_by: checkpoint.updated_by.clone(),
                updated_at: checkpoint.updated_at.clone(),
            };

            return Ok(Json(resp));
        }
        Err(enums::DbError::InternalError) => {
            log::error!("Error while getting checkpoint from DB");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Checkpoint with UUID '{checkpoint_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };
}
