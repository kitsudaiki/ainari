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
use std::str::FromStr;
use uuid::Uuid;

use crate::database::cluster_table;
use crate::database::task_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "task",
    summary = "Abort task",
    description = r###"Abort a task."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn abort_task(
    uuids: Path<(Uuid, Uuid)>,
    context: UserContext,
) -> Result<Json<TaskResp>, ErrorResponse> {
    let (cluster_uuid, task_uuid) = uuids.into_inner();

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

    // check current state of the task to avoid to abort finished and errored tasks
    match task_table::get_task(&task_uuid, &cluster_uuid, &context) {
        Ok(task_data) => {
            if task_data.task_state == TaskState::Aborted.to_string()
                || task_data.task_state == TaskState::Error.to_string()
                || task_data.task_state == TaskState::Finished.to_string()
            {
                let state_str = task_data.task_state.to_string();
                let msg = format!(
                    "Task with UUID '{task_uuid}' is in state '{state_str}' can not be aborted."
                );
                return Err(ErrorResponse::Conflict(msg));
            }
        }
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Task with UUID '{task_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // set abort-state in database
    match task_table::update_task_state(&task_uuid, &TaskState::Aborted) {
        Ok(()) => {}
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Task with UUID '{task_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    }

    // get task from database
    let task_data = match task_table::get_task(&task_uuid, &cluster_uuid, &context) {
        Ok(task_data) => task_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Task with UUID '{task_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // convert enums
    let task_type = match TaskType::from_str(task_data.task_type.as_str()) {
        Ok(task_type) => task_type,
        Err(()) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
    let task_state = match TaskState::from_str(task_data.task_state.as_str()) {
        Ok(task_state) => task_state,
        Err(()) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let resp = TaskResp {
        uuid: task_uuid,
        name: task_data.name.clone(),
        task_type,
        state: task_state,
        total_number_of_epochs: task_data.total_number_of_epochs,
        current_epoch: task_data.current_epoch,
        total_number_of_cycles: task_data.total_number_of_cycles,
        current_cycle: task_data.current_cycle,
        queued_at: task_data.queued_at.clone(),
        started_at: task_data.started_at.clone(),
        finished_at: task_data.finished_at.clone(),
        error_message: task_data.error_message.clone(),
        created_by: task_data.created_by.clone(),
        created_at: task_data.created_at.clone(),
    };

    return Ok(Json(resp));
}
