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

use crate::config;
use crate::core::processing::tasks::{
    CloudHypervisorVirtualMachineCreateInfo, Task, TaskMeta, TaskVariant,
};
use crate::database::task_table;
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;

#[api_operation(
    tag = "task",
    summary = "Create new virtual_machine",
    description = r###"Create a new task, which creates the virtual_machine.

The image and the public-key of the request are stored on the reserved
virtual_machine, before the task is queued."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn create_virtual_machine(
    body: Json<VirtualMachineCreateTaskReq>,
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::VirtualMachineCreate;

    // the virtual_machine was reserved without an image and without a public-key, so both are
    // stored now, before the task, which creates the virtual_machine, reads them again
    virtual_machine_table::set_virtual_machine_image(
        &virtual_machine_uuid,
        &body.image_uuid,
        &body.public_key_uuid,
        &context,
    )
    .map_err(|e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e))?;

    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context).map_err(
            |e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e),
        )?;

    // prepare task-info
    let task_name = format!(
        "Create virtual machine with UUID {}",
        virtual_machine_data.uuid
    );
    let info = CloudHypervisorVirtualMachineCreateInfo {
        vm_uuid: virtual_machine_data.uuid,
        name: task_name.clone(),
        context: context.clone(),
    };

    let _endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // create new task
    let task = Task {
        uuid: task_uuid,
        resouce_uuid: *virtual_machine_uuid,
        resource_type: TaskResourceType::VirtualMachine,
        name: task_name.clone(),
        info: TaskVariant::CloudHypervisorVirtualMachineCreate(info),
        meta: TaskMeta::new(),
    };
    super::super::task::add_task(task, &task_type, &context).inspect_err(|e| {
        log::error!("Creating a train-task failed with error: {e}");
        // in case of an error, delete the temp-directory with all downlaoded files of this task again
        // TODO: revert all
        // super::remove_all(&temp_dir);
    })?;

    // get new created task from database to get additional information
    let task_data = task_table::get_task(&task_uuid, &virtual_machine_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;

    let resp = TaskResp {
        uuid: task_uuid,
        name: task_data.name,
        task_type: task_data.task_type,
        state: task_data.task_state,
        queued_at: task_data.queued_at,
        started_at: task_data.started_at,
        finished_at: task_data.finished_at,
        messages: task_data.messages,
        created_by: task_data.created_by,
        created_at: task_data.created_at,
    };

    Ok(CreatedJson(resp))
}
