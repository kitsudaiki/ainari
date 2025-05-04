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

use std::fs;
use log::error;

use apistos::actix::NoContent;
use apistos::api_operation;
use actix_web::web::Path;
use uuid::Uuid;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::checkpoint_table;

use hanami_common::enums;

#[api_operation(
    tag = "checkpoint",
    summary = "Delete checkpoint",
    description = r###"Delete a checkpoint from the database and core."###,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_checkpoint(checkpoint_uuid: Path<Uuid>, context: UserContext) -> Result<NoContent, ErrorResponse> {
    let checkpoint = match checkpoint_table::get_checkpoint(&checkpoint_uuid, &context) {
        Ok(checkpoint) => checkpoint,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };

    match fs::remove_file(&checkpoint.file_path) {
        Ok(_) => {},
        Err(_) => {
            error!("Failed to delete file '{}' from disc", checkpoint.file_path);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    match checkpoint_table::delete_checkpoint(&checkpoint_uuid, &context) {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };

    Ok(NoContent)   
}