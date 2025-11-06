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
use crate::database::dataset_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;
use ainari_common::enums;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "dataset",
    summary = "Initialize new dataset",
    description = r###"Initialize  new dataset. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn init_dataset(
    body: Json<DatasetCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<DatasetInternalResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    check_quota(&context).await?;

    let name = &body.name;
    let dataset_uuid = &body.uuid.clone();

    let upload_dir_path = config::CONFIG.storage.dataset_location.clone();
    let upload_dir = PathBuf::from(&upload_dir_path);
    let target_filepath: PathBuf = upload_dir.join(dataset_uuid.to_string());
    let file_path_str: String = target_filepath.to_string_lossy().into();

    // add new dataset to datbase
    match dataset_table::add_new_dataset(dataset_uuid, name, &file_path_str, &context) {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("Failed to add dataset to database: {e}");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError(msg));
        }
    }

    // get new created dataset from database to get addtional information
    match dataset_table::get_dataset(dataset_uuid, &context) {
        Ok(dataset) => {
            let resp = DatasetInternalResp {
                uuid: *dataset_uuid,
                name: dataset.name.clone(),
                file_path: dataset.file_path.clone(),
                created_by: dataset.created_by.clone(),
                created_at: dataset.created_at.clone(),
                updated_by: dataset.updated_by.clone(),
                updated_at: dataset.updated_at.clone(),
            };

            return Ok(CreatedJson(resp));
        }
        Err(enums::DbError::InternalError) => {
            log::error!("Error while getting dataset from DB");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{dataset_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };
}

async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of datasets of the user
    let current_number_of_datasets = match dataset_table::count_datasets(context) {
        Ok(number) => number,
        Err(e) => {
            log::error!("Failed to count datasets in database.: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check the maximum number of datasets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let max_number_of_datasets = match get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body.max_dataset as i64,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check if quota is already exceeded
    if current_number_of_datasets as i64 >= max_number_of_datasets {
        return Err(ErrorResponse::Conflict(
            "Maximum number of datasets exceeded.".to_string(),
        ));
    }

    Ok(())
}
