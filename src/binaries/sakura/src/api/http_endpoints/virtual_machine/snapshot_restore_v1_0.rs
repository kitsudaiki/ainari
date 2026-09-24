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
    CloudHypervisorVirtualMachineRestoreInfo, Task, TaskMeta, TaskVariant,
};
use crate::database::task_table;
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::image::get_image;

#[api_operation(
    tag = "task",
    summary = "Create new snapshot-restore-task",
    description = r###"Create a new task, which resets the root-disk of an existing virtual_machine to
the state of a snapshot.

Only images, which are marked as snapshot, can be restored.

The virtual_machine is shut down, while its root-disk is replaced, and booted again afterwards."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn snapshot_restore_task(
    body: Json<TaskSnapshotRestoreReq>,
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::SnapshotRestore;

    // check if virtual_machine exist
    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context).map_err(
            |e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e),
        )?;

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // check if the image exist and is a snapshot, so an invalid image is rejected before the
    // task is queued
    let image_resp = get_image(
        &endpoints.ryokan,
        &context.token,
        &config::INTERNAL_API_KEY,
        &body.image_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;
    if !image_resp.is_snapshot {
        return Err(ErrorResponse::BadRequest(format!(
            "Image {} is not a snapshot and can not be restored.",
            image_resp.uuid
        )));
    }

    // prepare task-info
    let task_description = format!(
        "Restore snapshot-image {} into virtual machine with UUID {}",
        image_resp.uuid, virtual_machine_data.uuid
    );
    let info = CloudHypervisorVirtualMachineRestoreInfo {
        vm_uuid: virtual_machine_data.uuid,
        image_uuid: image_resp.uuid,
        description: task_description.clone(),
        context: context.clone(),
    };

    // create new task, which is processed by the same worker-thread as all other tasks of the
    // virtual_machine, so the restore can not overtake its creation
    let task = Task {
        uuid: task_uuid,
        resouce_uuid: *virtual_machine_uuid,
        resource_type: TaskResourceType::VirtualMachine,
        description: task_description,
        info: TaskVariant::CloudHypervisorVirtualMachineRestore(info),
        meta: TaskMeta::new(),
    };
    super::super::task::add_task(task, &task_type, &context)
        .inspect_err(|e| log::error!("Creating a restore-task failed with error: {e}"))?;

    // get new created task from database to get additional information
    let task_data = task_table::get_task(&task_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e))?;

    let resp = TaskResp {
        uuid: task_uuid,
        description: task_data.description,
        task_type: task_data.task_type,
        state: task_data.task_state,
        queued_at: task_data.queued_at,
        started_at: task_data.started_at,
        finished_at: task_data.finished_at,
        messages: task_data.messages,
        created_by: task_data.created_by,
    };

    Ok(CreatedJson(resp))
}
