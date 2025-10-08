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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use std::path::PathBuf;
use validator::Validate;

use crate::config;
use crate::database::checkpoint_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::checkpoint_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "checkpoint",
    summary = "Initialize new checkpoint",
    description = r###"Initialize  new checkpoint. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn init_checkpoint(
    body: Json<CheckpointCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<CheckpointInternalResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    let name = &body.name;
    let checkpoint_uuid = &body.uuid.clone();

    let upload_dir_path = config::CONFIG.storage.checkpoint_location.clone();
    let upload_dir = PathBuf::from(&upload_dir_path);
    let target_filepath: PathBuf = upload_dir.join(checkpoint_uuid.to_string());
    let file_path_str: String = target_filepath.to_string_lossy().into();

    // add new checkpoint to datbase
    match checkpoint_table::add_new_checkpoint(checkpoint_uuid, name, &file_path_str, &context) {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("Failed to add checkpoint to database: {e}");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError(msg));
        }
    }

    // get new created checkpoint from database to get addtional information
    match checkpoint_table::get_checkpoint(checkpoint_uuid, &context) {
        Ok(checkpoint) => {
            let resp = CheckpointInternalResp {
                uuid: *checkpoint_uuid,
                name: checkpoint.name.clone(),
                file_path: checkpoint.file_path.clone(),
                created_by: checkpoint.created_by.clone(),
                created_at: checkpoint.created_at.clone(),
                updated_by: checkpoint.updated_by.clone(),
                updated_at: checkpoint.updated_at.clone(),
            };

            return Ok(CreatedJson(resp));
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
