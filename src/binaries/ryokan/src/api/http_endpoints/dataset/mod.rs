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

pub mod check_dataset_v1_0;
pub mod create_dataset_v1_0;
pub mod delete_dataset_v1_0;
pub mod get_dataset_internal_v1_0;
pub mod get_dataset_v1_0;
pub mod init_dataset_internal_v1_0;
pub mod list_dataset_v1_0;

use uuid::Uuid;

use crate::config;
use crate::database::dataset_table;

use ainari_api::common_functions::map_ainari_error_to_api_response;
use ainari_api::common_functions::{convert_uuid, get_endpoints};
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::dataset_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;
use ainari_clients::secret::{generate_secret, get_secret_payload};
use ainari_common::enums;
use ainari_common::secret::Secret;

/// Validates that the provided dataset type is one of the supported types.
///
/// This function checks if the given `dataset_type` is either "mnist" or "csv".
/// If the type is not in the supported list, it returns an `ErrorResponse::BadRequest`
/// with a descriptive message. Otherwise, it returns `Ok(())` indicating the type is valid.
///
/// # Arguments
///
/// * `dataset_type` - A string slice containing the dataset type to validate.
///
/// # Returns
///
/// * `Ok(())` - If the dataset type is valid ("mnist" or "csv").
/// * `Err(ErrorResponse::BadRequest)` - If the dataset type is not in the supported list.
///
fn check_dataset_type(dataset_type: &String) -> Result<(), ErrorResponse> {
    if !["mnist", "csv"].contains(&dataset_type.as_str()) {
        let msg = format!("Type '{dataset_type}' is not in list [ mnist, csv ]");
        return Err(ErrorResponse::BadRequest(msg.to_string()));
    }

    Ok(())
}

/// Checks if the user has reached their dataset quota limit.
///
/// # Arguments
///
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(())` - If the user is within their dataset quota limit
///
/// * `Err(ErrorResponse::Unauthorized)` - If the user is not authorized to check their quota
/// * `Err(ErrorResponse::BadRequest)` - If the input to the quota check is invalid
/// * `Err(ErrorResponse::Conflict)` - If the user has exceeded their dataset quota
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error checking the quota
///
async fn check_dataset_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of datasets of the user
    let current_number_of_datasets = match dataset_table::count_datasets(context) {
        Ok(number) => number,
        Err(e) => {
            log::error!("Failed to count datasets in database.: {e}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // check the maximum number of datasets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    let max_number_of_datasets = quota.max_dataset as i64;
    // check if quota is already exceeded
    if current_number_of_datasets as i64 >= max_number_of_datasets {
        return Err(ErrorResponse::Conflict(
            "Maximum number of datasets exceeded.".to_string(),
        ));
    }

    Ok(())
}

/// Adds a new dataset to the database.
///
/// # Arguments
///
/// * `dataset_uuid` - A reference to the UUID of the dataset to be added
/// * `dataset_name` - A reference to the name of the dataset
/// * `onsen_address` - A reference to the onsen address associated with the dataset
/// * `file_path` - A reference to the file path where the dataset is stored
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(())` - If the dataset was successfully added to the database
///
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error adding the dataset
///
fn add_dataset_to_database(
    dataset_uuid: &Uuid,
    dataset_name: &str,
    onsen_address: &str,
    file_path: &str,
    secret_uuid: &Uuid,
    number_of_rows: i64,
    number_of_columns: i64,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    // add new dataset to datbase
    match dataset_table::add_new_dataset(
        dataset_uuid,
        dataset_name,
        onsen_address,
        file_path,
        secret_uuid,
        number_of_rows,
        number_of_columns,
        context,
    ) {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("Failed to add dataset to database: {e}");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError(msg));
        }
    }

    Ok(())
}

/// Retrieves a dataset by its UUID and returns its information.
///
/// # Arguments
///
/// * `dataset_uuid` - A reference to the UUID of the dataset to retrieve
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(DatasetResp)` - A response object containing dataset information if successful
///
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error retrieving the dataset
/// * `Err(ErrorResponse::NotFound)` - If the dataset with the given UUID was not found
///
async fn get_dataset(
    dataset_uuid: &Uuid,
    context: &UserContext,
) -> Result<DatasetResp, ErrorResponse> {
    let dataset_data = match dataset_table::get_dataset(dataset_uuid, context) {
        Ok(dataset_data) => dataset_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{dataset_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let resp = DatasetResp {
        uuid: *dataset_uuid,
        name: dataset_data.name.clone(),
        number_of_rows: dataset_data.number_of_rows as u64,
        number_of_columns: dataset_data.number_of_columns as u64,
        created_by: dataset_data.created_by.clone(),
        created_at: dataset_data.created_at.clone(),
        updated_by: dataset_data.updated_by.clone(),
        updated_at: dataset_data.updated_at.clone(),
    };

    Ok(resp)
}

/// Retrieves a dataset from the database based on its UUID.
///
/// # Arguments
///
/// * `dataset_uuid` - A reference to the UUID of the dataset to retrieve
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(DatasetInternalResp)` - A response object containing the dataset information if found
///
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error accessing the database
/// * `Err(ErrorResponse::NotFound)` - If no dataset was found with the provided UUID
///
fn get_dataset_internal(
    dataset_uuid: &Uuid,
    context: &UserContext,
) -> Result<DatasetInternalResp, ErrorResponse> {
    // get new created dataset from database to get addtional information
    match dataset_table::get_dataset(dataset_uuid, context) {
        Ok(dataset) => {
            let secret_uuid = convert_uuid(&dataset.secret_uuid)?;
            let resp = DatasetInternalResp {
                uuid: *dataset_uuid,
                name: dataset.name.clone(),
                onsen_address: dataset.onsen_address.clone(),
                file_path: dataset.file_path.clone(),
                number_of_rows: dataset.number_of_rows as u64,
                number_of_columns: dataset.number_of_columns as u64,
                secret_uuid,
                created_by: dataset.created_by.clone(),
                created_at: dataset.created_at.clone(),
                updated_by: dataset.updated_by.clone(),
                updated_at: dataset.updated_at.clone(),
            };

            Ok(resp)
        }
        Err(enums::DbError::InternalError) => {
            log::error!("Error while getting dataset from DB");
            Err(ErrorResponse::InternalError("Internal Error".to_string()))
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Dataset with UUID '{dataset_uuid}' not found.");
            Err(ErrorResponse::NotFound(msg))
        }
    }
}

async fn generate_new_key(
    dataset_uuid: &Uuid,
    context: &UserContext,
) -> Result<(Uuid, Secret), ErrorResponse> {
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.insecure_clients).await?;
    let secret_name = format!("autogenerated secret for dataset {dataset_uuid}");

    let secret_meta = generate_secret(
        &endpoints.omamori,
        &context.token,
        &secret_name,
        config::CONFIG.insecure_clients,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    let secret_payload = get_secret_payload(
        &endpoints.omamori,
        &context.token,
        &secret_meta.uuid,
        config::CONFIG.insecure_clients,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok((
        secret_meta.uuid,
        Secret::from(secret_payload.secret_payload),
    ))
}
