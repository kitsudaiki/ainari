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
use std::path::PathBuf;
use uuid::Uuid;

use crate::database::dataset_table;

use ainari_api::errors::ErrorResponse;
use ainari_api::structs::dataset_structs::DatasetResp;
use ainari_api::user_context::UserContext;
use ainari_common::enums;
use ainari_dataset::dataset_io::read_data_set_file;

#[api_operation(
    tag = "dataset",
    summary = "Get dataset",
    description = r###"Get information of a dataset from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_dataset(
    dataset_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<DatasetResp>, ErrorResponse> {
    let dataset_data = match dataset_table::get_dataset(&dataset_uuid, &context) {
        Ok(dataset_data) => dataset_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{dataset_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let file_handle = match read_data_set_file(&PathBuf::from(dataset_data.file_path)) {
        Ok(file_handle) => file_handle,
        Err(_) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let resp = DatasetResp {
        uuid: *dataset_uuid,
        name: dataset_data.name.clone(),
        number_of_rows: file_handle.get_number_of_rows(),
        number_of_columns: file_handle.header.columns.len() as u64,
        created_by: dataset_data.created_by.clone(),
        created_at: dataset_data.created_at.clone(),
        updated_by: dataset_data.updated_by.clone(),
        updated_at: dataset_data.updated_at.clone(),
    };

    return Ok(Json(resp));
}
