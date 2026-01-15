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

use actix_web::web::{Json, Path};
use apistos::api_operation;
use uuid::Uuid;

use crate::database::model_table;
use crate::database::task_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "task",
    summary = "List tasks",
    description = r###"List all tasks of a model"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn list_task(
    model_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<TaskListResp>, ErrorResponse> {
    // check if model exist
    model_table::get_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    let tasks = match task_table::list_tasks(&model_uuid, &context) {
        Ok(tasks) => tasks,
        Err(e) => {
            log::error!("Failed to get list of tasks form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let mut resp = TaskListResp { tasks: Vec::new() };

    for task in tasks {
        let uuid = convert_uuid(&task.uuid)?;
        let task_type = super::convert_task_type(&task.task_type)?;
        let task_state = super::convert_task_state(&task.task_state)?;

        let obj = TaskBasicResp {
            uuid,
            name: task.name,
            task_type,
            state: task_state,
            total_number_of_epochs: task.total_number_of_epochs,
            current_epoch: task.current_epoch,
        };

        resp.tasks.push(obj);
    }

    Ok(Json(resp))
}
