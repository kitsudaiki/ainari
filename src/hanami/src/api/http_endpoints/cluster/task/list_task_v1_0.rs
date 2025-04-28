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
use std::str::FromStr;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::task_table;

use hanami_common::enums;

use super::task_structs::{TaskBasicResp, TaskListResp, TaskType};

#[api_operation(
    tag = "task",
    summary = "Create new train-task",
    description = r###"Create new train-task for a task"###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_task(task_uuid: Path<Uuid>, context: UserContext) -> Result<Json<TaskListResp>, ErrorResponse> {
    let tasks = task_table::list_tasks(&context).unwrap();

    let mut resp = TaskListResp {
        tasks: Vec::new(),
    };

    for task in tasks {
        let uuid = match Uuid::parse_str(&task.uuid) {
            Ok(uuid) => uuid,
            Err(e) =>  return Err(ErrorResponse::InternalError("".to_string())),
        };

        // convert task-type
        let task_type = match TaskType::from_str(task.task_type.as_str()) {
            Ok(task_type) => task_type,
            Err(()) => {
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };
        
        let obj = TaskBasicResp {
            uuid: uuid,
            name: task.name.clone(),
            task_type: task_type,
        };

        resp.tasks.push(obj);
    }

    Ok(Json(resp))
}
