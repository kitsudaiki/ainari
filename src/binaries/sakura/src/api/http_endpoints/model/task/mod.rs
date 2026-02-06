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

pub mod abort_task_v1_0;
pub mod checkpoint_restore_task_v1_0;
pub mod checkpoint_save_task_v1_0;
pub mod create_request_task_v1_0;
pub mod create_train_task_v1_0;
pub mod get_task_v1_0;
pub mod list_task_v1_0;

use std::fs;
use std::str::FromStr;
use uuid::Uuid;

use crate::config;
use crate::core::model_handler;
use crate::core::model_handler::*;
use crate::core::processing::tasks::Task;
use crate::database::task_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::task_structs::{TaskState, TaskType};
use ainari_api_structs::user_context::UserContext;
use ainari_clients::dataset::*;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::onsen_file_transfer::*;
use ainari_clients::quota::get_quota;
use ainari_clients::secret::get_secret_payload;
use ainari_common::config::Endpoint;
use ainari_common::secret::Secret;
use ainari_dataset::dataset_io::{Column, DataSetFileReadHandle, read_data_set_file};
use ainari_dataset::file_encryption::decrypt_file;

/// Retrieves the current number of open tasks for a specific model.
///
/// This function queries the model interface to get the count of currently open tasks.
/// It handles cases where the model might not exist or lacks an interface.
///
/// # Arguments
/// * `model_uuid` - The UUID of the model to check
///
/// # Returns
/// * `Result<usize, ErrorResponse>` - The number of open tasks or an error
fn get_current_number_of_open_tasks(model_uuid: &Uuid) -> Result<usize, ErrorResponse> {
    // get model-handle
    let model_handler = model_handler::MODEL_HANDLER.read().expect("mutex poisoned");
    let model_handle = match model_handler.models.get(model_uuid) {
        Some(model_handle) => model_handle,
        None => return Err(ErrorResponse::InternalError("".to_string())),
    };
    let model_interface = if let Some(interface) = &model_handle.model_interface {
        interface
    } else {
        let msg = format!("Model with UUID '{model_uuid}' has not interface on the host.");
        return Err(ErrorResponse::NotFound(msg));
    };

    Ok(model_interface
        .lock()
        .expect("mutex poisoned")
        .get_number_open_tasks())
}

/// Checks if the task queue quota for a model is exceeded.
///
/// This asynchronous function verifies whether the current number of open tasks
/// for a model exceeds the user's quota. It queries the quota service and compares
/// it with the current task count.
///
/// # Arguments
/// * `model_uuid` - The UUID of the model to check
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<(), ErrorResponse>` - Success or an error if quota is exceeded
async fn check_task_queue_quota(
    model_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let current_number_of_open_tasks = get_current_number_of_open_tasks(model_uuid)?;

    // check the maximum number of secrets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // check if quota is already exceeded
    if current_number_of_open_tasks as i64 >= quota.max_taskqueue as i64 {
        let msg =
            format!("Maximum number of open tasks for model with UUID '{model_uuid}' exceeded.");
        return Err(ErrorResponse::Conflict(msg));
    }

    Ok(())
}

/// Converts a string representation of a task type to its enum variant.
///
/// # Arguments
/// * `task_type` - The string to convert
///
/// # Returns
/// * `Result<TaskType, ErrorResponse>` - The converted task type or an error
fn convert_task_type(task_type: &String) -> Result<TaskType, ErrorResponse> {
    let converted_task_type = TaskType::from_str(task_type.as_str()).map_err(|_| {
        log::error!("Failed to convert task-type '{task_type}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(converted_task_type)
}

/// Converts a string representation of a task state to its enum variant.
///
/// # Arguments
/// * `task_state` - The string to convert
///
/// # Returns
/// * `Result<TaskState, ErrorResponse>` - The converted task state or an error
pub fn convert_task_state(task_state: &String) -> Result<TaskState, ErrorResponse> {
    let converted_task_state = TaskState::from_str(task_state.as_str()).map_err(|_| {
        log::error!("Failed to convert task-state '{task_state}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(converted_task_state)
}

/// Adds a new task to a model and stores it in the database.
///
/// This function handles both the database storage and the model interface registration
/// of a new task. It ensures the task is properly tracked in both places.
///
/// # Arguments
/// * `task` - The task to add
/// * `task_type` - The type of the task
/// * `context` - The user context for database operations
///
/// # Returns
/// * `Result<(), ErrorResponse>` - Success or an error
fn add_task_to_model(
    task: Task,
    task_type: &TaskType,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let model_handler = model_handler::MODEL_HANDLER.read().expect("mutex poisoned");
    let model_handle = match model_handler.models.get(&task.model_uuid) {
        Some(model_handle) => model_handle,
        None => return Err(ErrorResponse::InternalError("".to_string())),
    };
    let model_interface = if let Some(interface) = &model_handle.model_interface {
        interface
    } else {
        let msg = format!(
            "Model with UUID '{}' has not interface on the host.",
            task.model_uuid
        );
        return Err(ErrorResponse::NotFound(msg));
    };

    task_table::add_new_task(
        &task.uuid,
        &task.model_uuid,
        &task.name,
        task_type,
        &task.meta.number_of_epochs,
        &task.meta.number_of_cycles,
        context,
    )
    .map_err(|e| {
        log::error!(
            "Failed to add task with UUID '{}' to database with error: {e}.",
            task.uuid
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    model_interface
        .lock()
        .expect("mutex poisoned")
        .add_task(task);

    Ok(())
}

/// Handles the output data from a task, preparing it for further processing.
///
/// This function manages the output buffer from the model and calculates
/// the column information for the output data.
///
/// # Arguments
/// * `output` - The task output to handle
/// * `model_uuid` - The UUID of the model
/// * `total_output_size` - The current total output size
///
/// # Returns
/// * `Result<(Column, u64), ErrorResponse>` - The column information and size
fn handle_output(
    output: &TaskDatasetResultLink,
    model_uuid: &Uuid,
    total_output_size: u64,
) -> Result<(Column, u64), ErrorResponse> {
    let model_handler = MODEL_HANDLER.read().expect("mutex poisoned");

    let size = {
        let output_buffer_mutex = model_handler
            .get_output_buffer(model_uuid, &output.hexagon)
            .map_err(map_ainari_error_to_api_response)?;
        let output_buffer = output_buffer_mutex.lock().expect("mutex poisoned");
        output_buffer.output_neurons.len() as u64
    };

    // HINT(kitsudaiki): drop lock here, because otherwise cargo clipply has a problem with the lock
    // in combination with the later coming await-call
    drop(model_handler);

    let col = Column {
        start: total_output_size,
        end: total_output_size + size,
    };

    Ok((col, size))
}

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

/// Checks if the provided I/O matches the model's expected I/O.
///
/// This function verifies that:
/// 1. All provided I/O matches the model's expected hexagon names
/// 2. All expected hexagon names have corresponding I/O provided
///
/// # Arguments
/// * `model_io` - The expected I/O hexagon names from the model
/// * `provided_io` - The provided I/O to check
///
/// # Returns
/// * `Result<(), ErrorResponse>` - Success or an error if I/O doesn't match
fn check_model_io<T>(model_io: &Vec<String>, provided_io: &Vec<T>) -> Result<(), ErrorResponse>
where
    T: DatasetLink,
{
    for link in provided_io {
        if !model_io.contains(&link.get_hexagon_name()) {
            let hexagon_name = &link.get_hexagon_name();
            let msg = format!("Hexagon with name {hexagon_name} not found in model");
            return Err(ErrorResponse::BadRequest(msg));
        }
    }

    for name in model_io {
        let mut found = false;
        for link in provided_io {
            if &link.get_hexagon_name() == name {
                found = true;
                break;
            }
        }

        if !found {
            let msg = format!("No data provided for hexagon {name}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    }

    Ok(())
}
