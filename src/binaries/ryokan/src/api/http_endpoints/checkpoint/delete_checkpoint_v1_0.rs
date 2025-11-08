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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::checkpoint_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::enums;

#[api_operation(
    tag = "checkpoint",
    summary = "Delete checkpoint",
    description = r###"Delete a checkpoint from the database and core."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_checkpoint(
    checkpoint_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let checkpoint = match checkpoint_table::get_checkpoint(&checkpoint_uuid, &context) {
        Ok(checkpoint) => checkpoint,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Checkpoint with UUID '{checkpoint_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    match onsen_file_transfer::delete_file(&checkpoint.onsen_address, &checkpoint.file_path).await {
        Ok(_) => {}
        Err(_) => {
            let onsen_address = checkpoint.onsen_address;
            let file_path = checkpoint.file_path;
            log::error!("Failed to delete file '{file_path}' from onsen '{onsen_address}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    match checkpoint_table::delete_checkpoint(&checkpoint_uuid, &context) {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            log::error!("Error while deleting checkpoint from DB");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Checkpoint with UUID '{checkpoint_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    Ok(NoContent)
}
