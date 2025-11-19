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
use uuid::Uuid;
use validator::Validate;

use super::check_task_queue_quota;
use crate::config;
use crate::core::processing::tasks::{CheckpointSaveInfo, Task, TaskMeta, TaskVariant};

use ainari_api::common_functions::get_endpoints;
use ainari_api::common_functions::map_ainari_error_to_api_response;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::checkpoint::*;

#[api_operation(
    tag = "task",
    summary = "Create new checkpoint-task",
    description = r###"Create new checkpoint-task for a cluster"###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn checkpoint_save_task(
    body: Json<TaskCheckpointSaveReq>,
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    check_task_queue_quota(&cluster_uuid, &context).await?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::CheckpointSave;

    // check if cluster exist
    let _ = super::super::get_cluster_from_database(&cluster_uuid, &context)?;

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.insecure_clients).await?;

    let checkpoint_create_resp = init_checkpoint(
        &endpoints.ryokan,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &task_uuid,
        &body.name,
        config::CONFIG.insecure_clients,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // prepare task-info
    let info = CheckpointSaveInfo {
        onsen_address: checkpoint_create_resp.onsen_address,
        file_path: checkpoint_create_resp.file_path,
    };

    // create new task
    let task = Task {
        uuid: task_uuid,
        cluster_uuid: *cluster_uuid,
        name: body.name.clone(),
        info: TaskVariant::CheckpointSave(info),
        meta: TaskMeta::new(1, 1, 1),
    };
    super::add_task_to_cluster(task, &task_type, &context)?;

    // get new created task from database to get addtional information
    let task_data = super::get_task_from_database(&task_uuid, &cluster_uuid, &context)?;
    let task_type = super::convert_task_type(&task_data.task_type)?;
    let task_state = super::convert_task_state(&task_data.task_state)?;

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

    return Ok(CreatedJson(resp));
}
