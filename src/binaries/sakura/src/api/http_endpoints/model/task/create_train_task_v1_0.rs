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
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use super::check_task_queue_quota;
use crate::config;
use crate::core::processing::tasks::{Task, TaskMeta, TaskVariant, TrainInfo};
use crate::database::model_table;
use crate::database::task_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;

#[api_operation(
    tag = "task",
    summary = "Create new train-task",
    description = r###"Create new train-task for a model"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_train_task(
    body: Json<TaskCreateTrainReq>,
    model_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_task_queue_quota(&model_uuid, &context).await?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::Train;
    let time_length = body.time_length.unwrap_or(1);
    let mut number_of_cycles = u64::MAX;

    if time_length < 1 {
        let msg = "Time-length must be 1 or bigger.".to_string();
        return Err(ErrorResponse::BadRequest(msg));
    }

    // check if model exist
    model_table::get_model(&model_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("model", &model_uuid, e))?;

    // create directory, where all temp-files of this operation are stored
    let temp_dir = format!(
        "{}/task_{}",
        config::CONFIG.storage.tempfile_location,
        task_uuid
    );
    create_directory(&temp_dir).await?;

    // prepare task-info
    let mut info = TrainInfo {
        inputs: HashMap::new(),
        outputs: HashMap::new(),
        temp_dir: temp_dir.clone(),
    };

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    {
        // prepare inputs for task
        for input in &body.inputs {
            let file_handle = super::handle_input(
                input,
                &endpoints.ryokan,
                &temp_dir,
                &context,
                &mut number_of_cycles,
            )
            .await?;
            info.inputs.insert(input.hexagon.clone(), file_handle);
        }

        // prepare outputs for task
        for output in &body.outputs {
            let file_handle = super::handle_input(
                output,
                &endpoints.ryokan,
                &temp_dir,
                &context,
                &mut number_of_cycles,
            )
            .await?;
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
            model_uuid: *model_uuid,
            name: body.name.clone(),
            info: TaskVariant::Training(info),
            meta: TaskMeta::new(number_of_cycles, body.number_of_epochs, time_length),
        };
        super::add_task_to_model(task, &task_type, &context)?;

        Ok(())
    }
    .inspect_err(|e| {
        log::error!("Creating a train-task failed with error: {e}");
        // in case of an error, delete the temp-directory with all downlaoded files of this task again
        super::remove_all(&temp_dir);
    })?;

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
