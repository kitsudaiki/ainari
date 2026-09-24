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

use actix_web::web::{Json, Query};
use apistos::api_operation;

use crate::database::task_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "task",
    summary = "List tasks",
    description = r###"List all tasks. With the query-parameter `resource_uuid`, only the tasks of
this resource, like a virtual_machine, are listed."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn list_task(
    query: Query<TaskListQuery>,
    context: UserContext,
) -> Result<Json<TaskListResp>, ErrorResponse> {
    let tasks = match task_table::list_tasks(&context, query.resource_uuid.as_ref()) {
        Ok(tasks) => tasks,
        Err(e) => {
            log::error!("Failed to get list of tasks form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let mut resp = TaskListResp { tasks: Vec::new() };

    for task in tasks {
        let obj = TaskBasicResp {
            uuid: task.uuid,
            description: task.description,
            task_type: task.task_type,
            state: task.task_state,
            queued_at: task.queued_at,
            started_at: task.started_at,
            finished_at: task.finished_at,
        };

        resp.tasks.push(obj);
    }

    Ok(Json(resp))
}
