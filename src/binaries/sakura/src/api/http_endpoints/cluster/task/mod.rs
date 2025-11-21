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
use crate::core::cluster_handler;
use crate::core::cluster_handler::*;
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

fn get_current_number_of_open_tasks(cluster_uuid: &Uuid) -> Result<usize, ErrorResponse> {
    // get cluster-handle
    let cluster_handler = cluster_handler::CLUSTER_HANDLER
        .read()
        .expect("mutex poisoned");
    let cluster_handle = match cluster_handler.clusters.get(cluster_uuid) {
        Some(cluster_handle) => cluster_handle,
        None => return Err(ErrorResponse::InternalError("".to_string())),
    };
    let cluster_interface = if let Some(interface) = &cluster_handle.cluster_interface {
        interface
    } else {
        let msg = format!("Cluster with UUID '{cluster_uuid}' has not interface on the host.");
        return Err(ErrorResponse::NotFound(msg));
    };

    Ok(cluster_interface
        .lock()
        .expect("mutex poisoned")
        .get_number_open_tasks())
}

async fn check_task_queue_quota(
    cluster_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let current_number_of_open_tasks = get_current_number_of_open_tasks(cluster_uuid)?;

    // check the maximum number of secrets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // check if quota is already exceeded
    if current_number_of_open_tasks as i64 >= quota.max_taskqueue as i64 {
        let msg = format!(
            "Maximum number of open tasks for cluster with UUID '{cluster_uuid}' exceeded."
        );
        return Err(ErrorResponse::Conflict(msg));
    }

    Ok(())
}

fn convert_task_type(task_type: &String) -> Result<TaskType, ErrorResponse> {
    let converted_task_type = TaskType::from_str(task_type.as_str()).map_err(|_| {
        log::error!("Failed to convert task-type '{task_type}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(converted_task_type)
}

fn convert_task_state(task_state: &String) -> Result<TaskState, ErrorResponse> {
    let converted_task_state = TaskState::from_str(task_state.as_str()).map_err(|_| {
        log::error!("Failed to convert task-state '{task_state}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(converted_task_state)
}

fn add_task_to_cluster(
    task: Task,
    task_type: &TaskType,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let cluster_handler = cluster_handler::CLUSTER_HANDLER
        .read()
        .expect("mutex poisoned");
    let cluster_handle = match cluster_handler.clusters.get(&task.cluster_uuid) {
        Some(cluster_handle) => cluster_handle,
        None => return Err(ErrorResponse::InternalError("".to_string())),
    };
    let cluster_interface = if let Some(interface) = &cluster_handle.cluster_interface {
        interface
    } else {
        let msg = format!(
            "Cluster with UUID '{}' has not interface on the host.",
            task.cluster_uuid
        );
        return Err(ErrorResponse::NotFound(msg));
    };

    task_table::add_new_task(
        &task.uuid,
        &task.cluster_uuid,
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

    cluster_interface
        .lock()
        .expect("mutex poisoned")
        .add_task(task);

    Ok(())
}

fn handle_output(
    output: &TaskDatasetResultLink,
    cluster_uuid: &Uuid,
    total_output_size: u64,
) -> Result<(Column, u64), ErrorResponse> {
    let cluster_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");

    let size = {
        let output_buffer_mutex = cluster_handler
            .get_output_buffer(cluster_uuid, &output.hexagon)
            .map_err(map_ainari_error_to_api_response)?;
        let output_buffer = output_buffer_mutex.lock().expect("mutex poisoned");
        output_buffer.output_neurons.len() as u64
    };

    // HINT(kitsudaiki): drop lock here, because otherwise cargo clipply has a problem with the lock
    // in combination with the later coming await-call
    drop(cluster_handler);

    let col = Column {
        start: total_output_size,
        end: total_output_size + size,
    };

    Ok((col, size))
}

async fn get_secret(secret_uuid: &Uuid, context: &UserContext) -> Result<Secret, ErrorResponse> {
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.insecure_clients)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let secret_payload = get_secret_payload(
        &endpoints.omamori,
        &context.token,
        secret_uuid,
        config::CONFIG.insecure_clients,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok(Secret::from(secret_payload.secret_payload))
}

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
        &config::CONFIG.api.internal_api_key,
        &input.dataset_uuid,
        config::CONFIG.insecure_clients,
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

fn remove_all(target_dir_path: &String) {
    // delete all temporary files
    let _ = std::fs::remove_dir_all(target_dir_path).map_err(|e| {
        log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
    });
}
