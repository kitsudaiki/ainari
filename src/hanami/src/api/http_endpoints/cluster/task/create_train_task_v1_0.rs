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

use hanami_core::cluster_handler;
use hanami_core::tasks::{Task, InternalTaskType, TaskVariant, TrainInfo};
use hanami_common::enums;
use hanami_dataset::dataset_io::read_data_set_file;

use super::task_structs::{TaskCreateReq, TaskResp, TaskType};

#[api_operation(
    tag = "task",
    summary = "Create new train-task",
    description = r###"Create new train-task for a cluster"###,
    error_code = 401,
    error_code = 500
)]
pub async fn create_train_task(body: Json<TaskCreateReq>, cluster_uuid: Path<Uuid>, context: UserContext) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    let task_uuid = Uuid::new_v4();

    // check if cluster-uuid exist in database
    let _ = match cluster_table::get_cluster(&cluster_uuid) {
        Ok(cluster) => cluster,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };

    // get cluster-handle
    let mut cluster_handler = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    let cluster_handle = match cluster_handler.get(&cluster_uuid) {
        Some(cluster_handle) => cluster_handle,
        None => return Err(ErrorResponse::InternalError("".to_string()))
    };

    // prepare task-info
    let mut info = TrainInfo {
        inputs: HashMap::new(),
        outputs: HashMap::new(),
        number_of_cycles: 60000,
        current_cycle: 0,
        time_length: 1,
    };

    // prepare inputs for task
    for input in &body.inputs {
        let dataset = match dataset_table::get_dataset(&input.dataset_uuid) {
            Ok(dataset) => dataset,
            Err(_) => 
            {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        let file_path = dataset.file_path;

        let file_handle = match read_data_set_file(&PathBuf::from(file_path)) {
            Ok(file_handle) => file_handle,
            Err(_) => 
            {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };
        
        info.inputs.insert(input.hexagon.clone(), file_handle);
    }

    // prepare outputs for task
    for output in &body.outputs {
        let dataset = match dataset_table::get_dataset(&output.dataset_uuid) {
            Ok(dataset) => dataset,
            Err(_) => 
            {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        let file_path = dataset.file_path;

        let file_handle = match read_data_set_file(&PathBuf::from(file_path)) {
            Ok(file_handle) => file_handle,
            Err(_) => 
            {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };
        
        info.outputs.insert(output.hexagon.clone(), file_handle);
    }

    // add new task to database
    match task_table::add_new_task(
        &task_uuid, 
        &body.name, 
        &body.task_type.to_string(),
        &context.user_id) 
    {
        Ok(_) => {},
        Err(_) => {
            let msg = format!("Failed to add task with UUID '{}' to database.", task_uuid);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // create new task
    let task = Task {
        uuid: task_uuid.clone(),
        task_type: InternalTaskType::TrainTask,
        name: body.name.clone(),
        userId: context.user_id.clone(),
        projectId: context.project_id.clone(),
        info: TaskVariant::Training(info),
    };

    // add task to task-queue of the cluster
    cluster_handle.add_task(task);

    // get new created task from database to get addtional information
    let task_data = match task_table::get_task(&task_uuid) {
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

    let resp = TaskResp {
        uuid: task_uuid.clone(),
        name: task_data.name.clone(),
        task_type: task_type,
        created_by: task_data.created_by.clone(),
        created_at: task_data.created_at.clone(),
        updated_by: task_data.updated_by.clone(),
        updated_at: task_data.updated_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
