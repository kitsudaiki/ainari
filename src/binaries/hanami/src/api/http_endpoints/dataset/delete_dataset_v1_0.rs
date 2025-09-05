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
use std::fs;
use uuid::Uuid;

use crate::database::dataset_table;

use ainari_api::errors::ErrorResponse;
use ainari_api::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "dataset",
    summary = "Delete dataset",
    description = r###"Delete a dataset from the database and files from the storage."###,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_dataset(
    dataset_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let dataset = match dataset_table::get_dataset(&dataset_uuid, &context) {
        Ok(dataset) => dataset,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{dataset_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    match fs::remove_file(&dataset.file_path) {
        Ok(_) => {}
        Err(_) => {
            let file_path = dataset.file_path;
            log::error!("Failed to delete file '{file_path}' from disc");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    match dataset_table::delete_dataset(&dataset_uuid, &context) {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{dataset_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    Ok(NoContent)
}
