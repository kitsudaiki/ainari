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
use ainari_common::secret::Secret;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::core::processing::tasks::{
    CloudHypervisorVirtualMachineCreateInfo, Task, TaskMeta, TaskVariant,
};
use crate::database::task_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;

#[api_operation(
    tag = "task",
    summary = "Create new virtual_machine",
    description = r###"Create new train-task for a virtual_machine"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_virtual_machine(
    body: Json<TaskCreateTrainReq>,
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let vm_uuid = Uuid::new_v4();
    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::VirtualMachineCreate;

    // create directory, where all temp-files of this operation are stored
    let temp_dir = format!(
        "{}/task_{}",
        config::CONFIG.storage.tempfile_location,
        task_uuid
    );
    create_directory(&temp_dir).await?;

    // prepare task-info
    let info = CloudHypervisorVirtualMachineCreateInfo {
        vm_uuid,
        number_of_cores: 2,
        memory_size: 1_073_741_824,
        public_key: Secret::from(
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIF5WE9inFSDLr3GesaH0AYEVlGV1q//dCIYEHL2Ju/A6 neptune@nep-station",
        ),
    };

    let _endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    {
        // create new task
        let task = Task {
            uuid: task_uuid,
            resouce_uuid: *virtual_machine_uuid,
            resource_type: TaskResourceType::VirtualMachine,
            name: body.name.clone(),
            info: TaskVariant::CloudHypervisorVirtualMachineCreate(info),
            meta: TaskMeta::new(0, body.number_of_epochs, 0, 0),
        };
        super::super::task::add_task(task, &task_type, &context)?;

        Ok(())
    }
    .inspect_err(|e| {
        log::error!("Creating a train-task failed with error: {e}");
        // in case of an error, delete the temp-directory with all downlaoded files of this task again
        super::remove_all(&temp_dir);
    })?;

    // get new created task from database to get addtional information
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
