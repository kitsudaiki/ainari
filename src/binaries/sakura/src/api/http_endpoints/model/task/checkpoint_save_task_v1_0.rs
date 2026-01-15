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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use super::check_task_queue_quota;
use crate::api::http_endpoints::model::task::get_secret;
use crate::config;
use crate::core::processing::tasks::{CheckpointSaveInfo, Task, TaskMeta, TaskVariant};
use crate::database::model_table;
use crate::database::task_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::checkpoint::*;
use ainari_clients::endpoints::get_endpoints;

#[api_operation(
    tag = "task",
    summary = "Create new checkpoint-task",
    description = r###"Create new checkpoint-task for a model"###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn checkpoint_save_task(
    body: Json<TaskCheckpointSaveReq>,
    model_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_task_queue_quota(&model_uuid, &context).await?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::CheckpointSave;

    // check if model exist
    model_table::get_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let checkpoint_create_resp = init_checkpoint(
        &endpoints.ryokan,
        &context.token,
        &config::INTERNAL_API_KEY,
        &task_uuid,
        &body.name,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    let secret = get_secret(&checkpoint_create_resp.secret_uuid, &context).await?;

    // prepare task-info
    let info = CheckpointSaveInfo {
        onsen_address: checkpoint_create_resp.onsen_address,
        file_path: checkpoint_create_resp.file_path,
        secret,
    };

    // create new task
    let task = Task {
        uuid: task_uuid,
        model_uuid: *model_uuid,
        name: body.name.clone(),
        info: TaskVariant::CheckpointSave(info),
        meta: TaskMeta::new(1, 1, 1),
    };
    super::add_task_to_model(task, &task_type, &context)?;

    // get new created task from database to get addtional information
    let task_data = task_table::get_task(&task_uuid, &model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;
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

    Ok(CreatedJson(resp))
}
