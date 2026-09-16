// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod checkpoint_restore_v1_0;
pub mod checkpoint_save_v1_0;
pub mod create_virtual_machine_v1_0;
pub mod delete_virtual_machine_internal_v1_0;
pub mod get_virtual_machine_internal_v1_0;
pub mod list_virtual_machine_internal_v1_0;
pub mod reserve_virtual_machine_internal_v1_0;

use std::fs;
use uuid::Uuid;

use crate::config;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::dataset::*;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::onsen_file_transfer::*;
use ainari_clients::secret::get_secret_payload;
use ainari_common::config::Endpoint;
use ainari_common::secret::Secret;
use ainari_dataset::dataset_io::{DataSetFileReadHandle, read_data_set_file};
use ainari_dataset::file_encryption::decrypt_file;

/// Retrieves a secret from the secret service.
///
/// This asynchronous function fetches secret payload information from the
/// secret service using the provided UUID and user context.
///
/// # Arguments
/// * `secret_uuid` - The UUID of the secret to retrieve
/// * `context` - The user context for authentication
///
/// # Returns
/// * `Result<Secret, ErrorResponse>` - The retrieved secret or an error
async fn get_secret(secret_uuid: &Uuid, context: &UserContext) -> Result<Secret, ErrorResponse> {
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let secret_payload = get_secret_payload(
        &endpoints.omamori,
        &context.token,
        secret_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok(Secret::from(secret_payload.secret_payload))
}

/// Handles the input data for a task, downloading, decrypting, and preparing it.
///
/// This asynchronous function manages the entire input data pipeline:
/// 1. Gets dataset information
/// 2. Downloads the encrypted dataset
/// 3. Decrypts the dataset
/// 4. Prepares the dataset for processing
///
/// # Arguments
/// * `input` - The task input to handle
/// * `endpoint` - The endpoint to use for dataset operations
/// * `temp_dir` - The temporary directory for file operations
/// * `context` - The user context for authentication
/// * `number_of_cycles` - The number of cycles to potentially adjust
///
/// # Returns
/// * `Result<DataSetFileReadHandle, ErrorResponse>` - The prepared dataset handle
#[allow(dead_code)]
async fn handle_input(
    input: &TaskDatasetLink,
    endpoint: &Endpoint,
    temp_dir: &String,
    context: &UserContext,
    number_of_cycles: &mut u64,
) -> Result<DataSetFileReadHandle, ErrorResponse> {
    // get dataset information
    let dataset_resp = get_dataset(
        endpoint,
        &context.token,
        &config::INTERNAL_API_KEY,
        &input.dataset_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // check if requested column even exist in the dataset
    if !dataset_resp.column_names.contains(&input.dataset_column) {
        let msg = format!(
            "Dataset-column with name '{}' doesn't exist in dataset with UUID '{}'",
            input.dataset_column, input.dataset_uuid
        );
        return Err(ErrorResponse::BadRequest(msg));
    }

    // create temp-file-paths
    let local_file_path = format!("{}/{}", temp_dir, dataset_resp.uuid);
    let local_encrypted_file_path = format!("{local_file_path}_encrypted");

    download_file(
        &dataset_resp.onsen_address,
        &dataset_resp.file_path,
        &local_encrypted_file_path,
    )
    .await
    .map_err(|e| {
        let _ = fs::remove_file(&local_encrypted_file_path);
        log::error!("Failed to download dataset-file from onsen: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // decrypt dataset
    let secret = get_secret(&dataset_resp.secret_uuid, context).await?;
    decrypt_file(&local_encrypted_file_path, &local_file_path, &secret)
        .await
        .map_err(|e| {
            let _ = fs::remove_file(&local_encrypted_file_path);
            let _ = fs::remove_file(&local_file_path);
            map_ainari_error_to_api_response(e)
        })?;

    // delete encrypted file again
    let _ = fs::remove_file(&local_encrypted_file_path);

    let mut file_handle = read_data_set_file(&local_file_path).map_err(|e| {
        log::error!(
            "Failed to read dataset-file '{}' with error: {e}",
            dataset_resp.file_path
        );
        let _ = fs::remove_file(&local_encrypted_file_path);
        let _ = fs::remove_file(&local_file_path);
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let number_of_rows = file_handle.get_number_of_rows();
    if *number_of_cycles > number_of_rows {
        *number_of_cycles = number_of_rows;
    }
    file_handle.selected_column = input.dataset_column.clone();

    Ok(file_handle)
}

/// Removes all files and directories in the specified target directory.
///
/// This function performs a complete cleanup of the specified directory,
/// removing all files and subdirectories within it.
///
/// # Arguments
/// * `target_dir_path` - The path to the directory to remove
fn remove_all(target_dir_path: &String) {
    // delete all temporary files
    let _ = std::fs::remove_dir_all(target_dir_path).map_err(|e| {
        log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
    });
}
