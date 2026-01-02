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
use crate::api::http_endpoints::cluster::task::get_secret;
use crate::config;
use crate::core::processing::tasks::{RequestInfo, Task, TaskMeta, TaskVariant};
use crate::database::cluster_table;
use crate::database::task_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::dataset::*;
use ainari_clients::endpoints::get_endpoints;
use ainari_common::config::Endpoint;
use ainari_dataset::dataset_io::{
    Column, DataSetFileWriteHandle, DataSetType, DatasetLink, init_new_data_set_file,
};

#[api_operation(
    tag = "task",
    summary = "Create new request-task",
    description = r###"Create new request-task for a cluster"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_request_task(
    body: Json<TaskCreateRequestReq>,
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_task_queue_quota(&cluster_uuid, &context).await?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::Request;
    let time_length = body.time_length.unwrap_or(1);
    let mut columns: HashMap<String, Column> = HashMap::new();
    let mut number_of_cycles = u64::MAX;

    if time_length < 1 {
        let msg = "Time-length must be 1 or bigger.".to_string();
        return Err(ErrorResponse::BadRequest(msg));
    }

    // check if cluster exist
    cluster_table::get_cluster(&cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("cluster", &cluster_uuid, e))?;

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // create directory, where all temp-files of this operation are stored
    let temp_dir = format!(
        "{}/task_{}",
        config::CONFIG.storage.tempfile_location,
        task_uuid
    );
    create_directory(&temp_dir).await?;

    {
        // prepare outputs for task
        let mut total_output_size: u64 = 0;
        for output in &body.results {
            let (col, size) = super::handle_output(output, &cluster_uuid, total_output_size)?;
            columns.insert(output.hexagon.clone(), col);
            total_output_size += size;
        }

        // initialize datbase for output
        let (result_file_handle, secret_uuid) = init_output_dataset(
            &endpoints.ryokan,
            &context.token,
            &task_uuid,
            &body.name,
            total_output_size,
            columns,
            &temp_dir,
        )
        .await?;

        let secret = get_secret(&secret_uuid, &context).await?;

        // prepare task-info
        let mut info = RequestInfo {
            inputs: HashMap::new(),
            results: result_file_handle,
            output_secret: secret,
            temp_dir: temp_dir.clone(),
        };

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
            info: TaskVariant::Request(Box::new(info)),
            meta: TaskMeta::new(number_of_cycles, 1, time_length),
        };
        super::add_task_to_cluster(task, &task_type, &context)?;

        Ok(())
    }
    .inspect_err(|e| {
        log::error!("Creating a request-task failed with error: {e}");
        // in case of an error, delete the temp-directory with all downlaoded files of this task again
        super::remove_all(&temp_dir);
    })?;

    // get new created task from database to get addtional information
    let task_data = task_table::get_task(&task_uuid, &cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;
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

    Ok(CreatedJson(resp))
}

async fn init_output_dataset(
    ryokan_endpoint: &Endpoint,
    token: &String,
    task_uuid: &Uuid,
    name: &str,
    total_output_size: u64,
    columns: HashMap<String, Column>,
    temp_dir: &String,
) -> Result<(DataSetFileWriteHandle, Uuid), ErrorResponse> {
    let result_path = format!("{}/task_result_{}", temp_dir, task_uuid);
    let result_path_encrypted = format!("{result_path}_encrypted");
    let description = task_uuid.to_string().clone();

    // filter colume-names
    let mut column_names: Vec<String> = Vec::new();
    for col in &columns {
        column_names.push(col.0.clone());
    }

    let dimension: (u64, Vec<String>) = (total_output_size, column_names);
    let dataset_create_resp = init_dataset_in_ryokan(
        ryokan_endpoint,
        token,
        &config::INTERNAL_API_KEY,
        task_uuid,
        name,
        dimension,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    let link = DatasetLink {
        onsen_address: dataset_create_resp.onsen_address,
        remote_file_path: dataset_create_resp.file_path,
        local_file_path: result_path,
        local_encrypted_file_path: result_path_encrypted,
    };

    // create new dataset for the resulting data
    let result_file_handle = match init_new_data_set_file(
        &link,
        *task_uuid,
        name,
        description,
        total_output_size,
        columns,
        DataSetType::FloatType,
    ) {
        Ok(file_handle) => file_handle,
        Err(_) => {
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let secret_uuid = dataset_create_resp.secret_uuid;

    Ok((result_file_handle, secret_uuid))
}
