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

use actix_web::web::Path;
use ainari_api_structs::task_structs::TaskState;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::core::model_handler;
use crate::database::model_table;
use crate::database::task_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "model",
    summary = "Delete model",
    description = r###"Delete a model from the database and core."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_model_internal(
    model_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // list all tasks
    let tasks = match task_table::list_tasks(&model_uuid, &context) {
        Ok(tasks) => tasks,
        Err(e) => {
            log::error!("Failed to get list of tasks form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // abort all open tasks
    for task in tasks {
        let uuid = convert_uuid(&task.uuid)?;
        let task_state = super::task::convert_task_state(&task.task_state)?;

        if task_state == TaskState::Created
            || task_state == TaskState::Queued
            || task_state == TaskState::Active
        {
            task_table::update_task_state(&uuid, &TaskState::Aborted)
                .map_err(|e| map_db_uuid_get_delete_error("task", &uuid, e))?;
        }
    }

    // prepare delete of model from core
    let model_handle = model_handler::CLUSTER_HANDLER
        .write()
        .expect("mutex poisoned");
    let model_interface = model_handle
        .get_model_interface(&model_uuid)
        .map_err(map_ainari_error_to_api_response)?;
    drop(model_handle);

    // stop the interface. This must be done outside of the model_handler to avoid a dead-lock
    let mut interface = model_interface.lock().expect("mutex poisoned");
    interface.stop();

    // delete model from core
    let mut model_handle = model_handler::CLUSTER_HANDLER
        .write()
        .expect("mutex poisoned");
    model_handle
        .delete_model(&model_uuid)
        .map_err(map_ainari_error_to_api_response)?;
    drop(model_handle);

    // delete model from database
    model_table::delete_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    Ok(NoContent)
}
