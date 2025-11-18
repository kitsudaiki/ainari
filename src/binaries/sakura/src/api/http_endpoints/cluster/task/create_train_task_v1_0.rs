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
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use super::check_task_queue_quota;
use crate::config;
use crate::core::processing::tasks::{Task, TaskMeta, TaskVariant, TrainInfo};

use ainari_api::common_functions::get_endpoints;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "task",
    summary = "Create new train-task",
    description = r###"Create new train-task for a cluster"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_train_task(
    body: Json<TaskCreateTrainReq>,
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
    let task_type = TaskType::Train;
    let time_length = body.time_length.unwrap_or(1);
    let mut number_of_cycles = u64::MAX;

    if time_length < 1 {
        let msg = "Time-length must be 1 or bigger.".to_string();
        return Err(ErrorResponse::BadRequest(msg));
    }

    // check if cluster exist
    let _ = super::super::get_cluster_from_database(&cluster_uuid, &context)?;

    // prepare task-info
    let mut info = TrainInfo {
        inputs: HashMap::new(),
        outputs: HashMap::new(),
    };

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.insecure_clients).await?;

    // prepare inputs for task
    for input in &body.inputs {
        let file_handle =
            super::handle_input(input, &endpoints.ryokan, &context, &mut number_of_cycles).await?;
        info.inputs.insert(input.hexagon.clone(), file_handle);
    }

    // prepare outputs for task
    for output in &body.outputs {
        let file_handle =
            super::handle_input(output, &endpoints.ryokan, &context, &mut number_of_cycles).await?;
        info.outputs.insert(output.hexagon.clone(), file_handle);
    }

    // handle the time-lenght-value
    if number_of_cycles < time_length {
        let msg = format!(
            "Time-length {time_length} is bigger than at least of of the seleced datasets."
        );
        return Err(ErrorResponse::BadRequest(msg));
    }
    number_of_cycles -= time_length - 1;

    // create new task
    let task = Task {
        uuid: task_uuid,
        cluster_uuid: *cluster_uuid,
        name: body.name.clone(),
        info: TaskVariant::Training(info),
        meta: TaskMeta::new(number_of_cycles, body.number_of_epochs, time_length),
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
