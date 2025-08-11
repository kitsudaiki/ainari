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

use actix_web::web::{Json, Path};
use apistos::actix::CreatedJson;
use apistos::api_operation;
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::core::cluster_handler;
use crate::core::processing::tasks::{CheckpointRestoreInfo, Task, TaskVariant};
use crate::database::checkpoint_table;
use crate::database::cluster_table;
use crate::database::task_table;

use ainari_common::enums;
use ainari_structs::task_structs::{TaskCheckpointRestoreReq, TaskResp, TaskState, TaskType};

#[api_operation(
    tag = "task",
    summary = "Create new checkpoint-task",
    description = r###"Create new checkpoint-task for a cluster"###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn checkpoint_restore_task(
    body: Json<TaskCheckpointRestoreReq>,
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {}", e);
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::CheckpointRestoreTask;

    // check if cluster exist
    match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // get checkpoint information from the database
    let checkpoint_data = match checkpoint_table::get_checkpoint(&body.checkpoint_uuid, &context) {
        Ok(checkpoint_data) => checkpoint_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Checkpoint with UUID '{}' not found", &body.checkpoint_uuid);
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // get cluster-handle
    let cluster_handler = cluster_handler::CLUSTER_HANDLER.read().unwrap();
    let cluster_handle = match cluster_handler.clusters.get(&cluster_uuid) {
        Some(cluster_handle) => cluster_handle,
        None => return Err(ErrorResponse::InternalError("".to_string())),
    };
    let cluster_interface = if let Some(interface) = &cluster_handle.cluster_interface {
        interface
    } else {
        let msg = format!("Cluster with UUID '{cluster_uuid}' has not interface on the host.");
        return Err(ErrorResponse::NotFound(msg));
    };

    // prepare task-info
    let info = CheckpointRestoreInfo {
        path: PathBuf::from(&checkpoint_data.file_path),
    };

    // add new task to database
    match task_table::add_new_task(
        &task_uuid,
        &cluster_uuid,
        &body.name,
        &task_type,
        &1,
        &1,
        &context,
    ) {
        Ok(_) => {}
        Err(_) => {
            log::error!("Failed to add task with UUID '{task_uuid}' to database.");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // create new task
    let task = Task {
        uuid: task_uuid.clone(),
        cluster_uuid: cluster_uuid.clone(),
        name: body.name.clone(),
        user_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        info: TaskVariant::CheckpointRestore(info),
    };
    cluster_interface.lock().unwrap().add_task(task);

    // get new created task from database to get addtional information
    let task_data = match task_table::get_task(&task_uuid, &cluster_uuid, &context) {
        Ok(task_data) => task_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
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
        error_message: task_data.error_message.clone(),
        created_by: task_data.created_by.clone(),
        created_at: task_data.created_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
