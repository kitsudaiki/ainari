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

use apistos::actix::CreatedJson;
use actix_web::web::{Json, Path};
use apistos::api_operation;
use log::error;
use uuid::Uuid;
use std::collections::HashMap;
use std::str::FromStr;
use std::path::PathBuf;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::cluster_table;
use crate::database::task_table;
use crate::database::dataset_table;
use crate::config;
use crate::core::cluster_handler;
use crate::core::tasks::{Task, TaskVariant, RequestInfo};

use hanami_common::enums;
use hanami_dataset::dataset_io::{init_new_data_set_file, read_data_set_file, DataSetType, Column};

use super::task_structs::{TaskCreateRequestReq, TaskResp, TaskType, TaskState};

#[api_operation(
    tag = "task",
    summary = "Create new request-task",
    description = r###"Create new request-task for a cluster"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_request_task(body: Json<TaskCreateRequestReq>, cluster_uuid: Path<Uuid>, context: UserContext) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::RequestTask;

    // check if cluster exist
    match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{}' not found.", cluster_uuid);
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // get cluster-handle
    let mut cluster_handler = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    let cluster_handle = match cluster_handler.get(&cluster_uuid) {
        Some(cluster_handle) => cluster_handle,
        None => return Err(ErrorResponse::InternalError("".to_string()))
    };

    // prepare task-info
    let mut info = RequestInfo {
        inputs: HashMap::new(),
        results: HashMap::new(),
        number_of_cycles: 0,
        time_length: 1,
    };

    let mut number_of_cycles =  u64::MAX;
    
    // prepare inputs for task
    for input in &body.inputs {
        let dataset = match dataset_table::get_dataset(&input.dataset_uuid, &context) {
            Ok(dataset) => dataset,
            Err(_) => {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        let file_path = dataset.file_path;

        match read_data_set_file(&PathBuf::from(file_path)) {
            Ok(file_handle) => {
                let number_of_rows = file_handle.get_number_of_rows();
                if number_of_cycles > number_of_rows {
                    number_of_cycles = number_of_rows;
                }

                info.inputs.insert(input.hexagon.clone(), file_handle);
            },
            Err(_) => {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };
    }

    info.number_of_cycles = number_of_cycles;

    // prepare outputs for task
    for output in &body.results {
        let result_uuid = Uuid::new_v4();
        let upload_dir_path = config::CONFIG.storage.dataset_location.clone();
        let upload_dir = PathBuf::from(&upload_dir_path);
        let target_filepath: PathBuf = upload_dir.join(&result_uuid.to_string());
        let description = "".to_string();
        let mut columns: HashMap<String, Column> = HashMap::new();
        let name = "poi".to_string();
        let row_size: u64 = 0;

        // add new dataset to datbase
        let file_path_str: String = target_filepath.to_string_lossy().into();
        match dataset_table::add_new_dataset(&result_uuid, &name, &file_path_str, &context) {
            Ok(_) => {},
            Err(_) => {
                let msg = format!("Failed to add dataset with ID '{result_uuid}' to database.");
                error!("{}", msg);
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        let col = Column {
            start: 0,
            end: 10,
        };
        columns.insert(output.hexagon.clone(),col);

        match init_new_data_set_file(
            &PathBuf::from(target_filepath), 
            result_uuid,
            name,
            description,
            row_size,
            columns,
            DataSetType::FloatType) 
        {
            Ok(file_handle) => {
                info.results.insert(output.hexagon.clone(), file_handle);
            },
            Err(_) => {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };
    }

    // add new task to database
    match task_table::add_new_task(
        &task_uuid, 
        &cluster_uuid,
        &body.name, 
        &task_type,
        &1, // TODO: corrent number of epochs
        &number_of_cycles,
        &context) 
    {
        Ok(_) => {},
        Err(_) => {
            error!("Failed to add task with UUID '{task_uuid}' to database.");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // create new task
    let task = Task {
        uuid: task_uuid.clone(),
        task_type: TaskType::RequestTask,
        name: body.name.clone(),
        user_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        info: TaskVariant::Request(info),
    };

    // add task to task-queue of the cluster
    cluster_handle.add_task(task);

    // get new created task from database to get addtional information
    let task_data = match task_table::get_task(&task_uuid, &cluster_uuid, &context) {
        Ok(task_data) => task_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };

    // convert task-type
    let task_type = match TaskType::from_str(task_data.task_type.as_str()) {
        Ok(task_type) => task_type,
        Err(()) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
    // convert task-state
    let task_state = match TaskState::from_str(task_data.task_state.as_str()) {
        Ok(task_state) => task_state,
        Err(()) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let resp = TaskResp {
        uuid: task_uuid.clone(),
        name: task_data.name.clone(),
        task_type: task_type,
        state: task_state,
        total_number_of_epochs: task_data.total_number_of_epochs.clone(),
        current_epoch: task_data.current_epoch.clone(),
        total_number_of_cycles: task_data.total_number_of_cycles.clone(),
        current_cycle: task_data.current_cycle.clone(),
        queued_at: task_data.queued_at.clone(),
        started_at: task_data.started_at.clone(),
        finished_at: task_data.finished_at.clone(),
        created_by: task_data.created_by.clone(),
        created_at: task_data.created_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
