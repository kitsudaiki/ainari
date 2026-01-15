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

use actix_web::web::Json;
use actix_web::web::Path;
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
    let (model_uuid, task_uuid) = uuids.into_inner();

    // check if model exist
    model_table::get_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    // check current state of the task to avoid to abort finished and errored tasks
    let old_task_data = task_table::get_task(&task_uuid, &model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;

    if old_task_data.task_state == TaskState::Aborted.to_string()
        || old_task_data.task_state == TaskState::Error.to_string()
        || old_task_data.task_state == TaskState::Finished.to_string()
    {
        let state_str = old_task_data.task_state.to_string();
        let msg =
            format!("Task with UUID '{task_uuid}' is in state '{state_str}' can not be aborted.");
        return Err(ErrorResponse::Conflict(msg));
    }

    // set abort-state in database
    task_table::update_task_state(&task_uuid, &TaskState::Aborted)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;

    // get task from database
    let task_data = task_table::get_task(&task_uuid, &model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;

    // convert enums
    let task_type = super::convert_task_type(&task_data.task_type)?;
    let task_state = super::convert_task_state(&task_data.task_state)?;

    let resp = TaskResp {
        uuid: task_uuid,
        name: task_data.name,
        task_type,
        state: task_state,
        total_number_of_epochs: task_data.total_number_of_epochs,
        current_epoch: task_data.current_epoch,
        total_number_of_cycles: task_data.total_number_of_cycles,
        current_cycle: task_data.current_cycle,
        queued_at: task_data.queued_at,
        started_at: task_data.started_at,
        finished_at: task_data.finished_at,
        error_message: task_data.error_message,
        created_by: task_data.created_by,
        created_at: task_data.created_at,
    };

    Ok(Json(resp))
}
