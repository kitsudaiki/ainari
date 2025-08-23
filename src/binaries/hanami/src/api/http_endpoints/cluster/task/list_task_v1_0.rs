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
use apistos::api_operation;
use std::str::FromStr;
use uuid::Uuid;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::cluster_table;
use crate::database::task_table;

use ainari_common::enums;
use ainari_structs::task_structs::{TaskBasicResp, TaskListResp, TaskState, TaskType};

#[api_operation(
    tag = "task",
    summary = "List tasks",
    description = r###"List all tasks of a cluster"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn list_task(
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<TaskListResp>, ErrorResponse> {
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

    let tasks = match task_table::list_tasks(&cluster_uuid, &context) {
        Ok(tasks) => tasks,
        Err(e) => {
            log::error!("Failed to get list of tasks form database: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let mut resp = TaskListResp { tasks: Vec::new() };

    for task in tasks {
        // parse-uuid-string coming from the database
        let uuid = match Uuid::parse_str(&task.uuid) {
            Ok(uuid) => uuid,
            Err(e) => {
                log::error!("Failed to convert task-uuid with error: '{e}'");
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        // convert task-type
        let task_type = match TaskType::from_str(task.task_type.as_str()) {
            Ok(task_type) => task_type,
            Err(()) => {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        // convert task-state
        let task_state = match TaskState::from_str(task.task_state.as_str()) {
            Ok(task_state) => task_state,
            Err(()) => {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        let obj = TaskBasicResp {
            uuid,
            name: task.name.clone(),
            task_type,
            state: task_state,
        };

        resp.tasks.push(obj);
    }

    Ok(Json(resp))
}
