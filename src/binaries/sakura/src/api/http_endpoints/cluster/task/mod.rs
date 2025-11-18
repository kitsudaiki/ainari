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

use std::str::FromStr;
use uuid::Uuid;

use crate::config;
use crate::core::cluster_handler;
use crate::core::cluster_handler::*;
use crate::core::processing::tasks::Task;
use crate::database::task_table::{self, TaskEntry};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::task_structs::{TaskState, TaskType};
use ainari_api_structs::user_context::UserContext;
use ainari_clients::dataset::*;
use ainari_clients::onsen_file_transfer::*;
use ainari_clients::quota::get_quota;
use ainari_common::config::Endpoint;
use ainari_common::enums;
use ainari_common::error::AinariError;
use ainari_dataset::dataset_io::{Column, DataSetFileReadHandle, read_data_set_file};

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
    let max_number_of_open_tasks = match get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body.max_taskqueue as i64,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // check if quota is already exceeded
    if current_number_of_open_tasks as i64 >= max_number_of_open_tasks {
        let msg = format!(
            "Maximum number of open tasks for cluster with UUID '{cluster_uuid}' exceeded."
        );
        return Err(ErrorResponse::Conflict(msg));
    }

    Ok(())
}

fn convert_task_type(task_type: &String) -> Result<TaskType, ErrorResponse> {
    let converted_task_type = match TaskType::from_str(task_type.as_str()) {
        Ok(converted_task_type) => converted_task_type,
        Err(()) => {
            log::error!("Failed to convert task-type '{task_type}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(converted_task_type)
}

fn convert_task_state(task_state: &String) -> Result<TaskState, ErrorResponse> {
    let converted_task_state = match TaskState::from_str(task_state.as_str()) {
        Ok(converted_task_state) => converted_task_state,
        Err(()) => {
            log::error!("Failed to convert task-state '{task_state}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(converted_task_state)
}

fn add_task_to_database(
    task_uuid: &Uuid,
    cluster_uuid: &Uuid,
    task_name: &str,
    task_type: &TaskType,
    total_number_of_epochs: &u64,
    total_number_of_cycles: &u64,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    match task_table::add_new_task(
        task_uuid,
        cluster_uuid,
        task_name,
        task_type,
        total_number_of_epochs,
        total_number_of_cycles,
        context,
    ) {
        Ok(_) => {}
        Err(_) => {
            log::error!("Failed to add task with UUID '{task_uuid}' to database.");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(())
}

fn get_task_from_database(
    task_uuid: &Uuid,
    cluster_uuid: &Uuid,
    context: &UserContext,
) -> Result<TaskEntry, ErrorResponse> {
    let task_data = match task_table::get_task(task_uuid, cluster_uuid, context) {
        Ok(task_data) => task_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };

    Ok(task_data)
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

    // get new created task from database to get addtional information
    add_task_to_database(
        &task.uuid,
        &task.cluster_uuid,
        &task.name,
        task_type,
        &task.meta.number_of_epochs,
        &task.meta.number_of_cycles,
        context,
    )?;

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

    let size = match cluster_handler.get_output_buffer(cluster_uuid, &output.hexagon) {
        Ok(output_buffer_mutex) => {
            let output_buffer = output_buffer_mutex.lock().expect("mutex poisoned");
            output_buffer.output_neurons.len() as u64
        }
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            let msg = format!("Invalid input: {msg}");
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
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

async fn handle_input(
    input: &TaskDatasetLink,
    endpoint: &Endpoint,
    context: &UserContext,
    number_of_cycles: &mut u64,
) -> Result<DataSetFileReadHandle, ErrorResponse> {
    // get dataset information
    let dataset_resp = match get_dataset(
        endpoint,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &input.dataset_uuid,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // TODO: change path
    let local_file_path = format!("/tmp/{}", dataset_resp.uuid);
    match download_file(
        &dataset_resp.onsen_address,
        &dataset_resp.file_path,
        &local_file_path,
    )
    .await
    {
        Ok(()) => {}
        Err(e) => {
            log::error!("Failed to download dataset-file from onsen: {e}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    }

    let file_handle = match read_data_set_file(&local_file_path) {
        Ok(mut file_handle) => {
            let number_of_rows = file_handle.get_number_of_rows();
            if *number_of_cycles > number_of_rows {
                *number_of_cycles = number_of_rows;
            }
            file_handle.selected_column = input.dataset_column.clone();

            file_handle
        }
        Err(e) => {
            log::error!(
                "Failed to read dataset-file '{}' with error: {e}",
                dataset_resp.file_path
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(file_handle)
}
