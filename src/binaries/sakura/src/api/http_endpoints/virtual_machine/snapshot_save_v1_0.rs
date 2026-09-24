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
    CloudHypervisorVirtualMachineSnapshotInfo, Task, TaskMeta, TaskVariant,
};
use crate::database::task_table;
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::snapshot::*;

#[api_operation(
    tag = "task",
    summary = "Create new snapshot-save-task",
    description = r###"Create a new task, which saves the root-disk of a virtual_machine as new snapshot.

The snapshot is registered in ryokan, before the task is queued, so the quota of the user is
checked immediately. The task encrypts the root-disk and uploads it into the onsen."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn snapshot_save_task(
    body: Json<TaskSnapshotSaveReq>,
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::SnapshotSave;

    // check if virtual_machine exist
    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context).map_err(
            |e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e),
        )?;

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // register the snapshot in ryokan, which also generates the secret for its encryption
    let snapshot_uuid = Uuid::new_v4();
    init_snapshot(
        &endpoints.ryokan,
        &context.token,
        &config::INTERNAL_API_KEY,
        &snapshot_uuid,
        &body.name,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // prepare task-info
    let task_name = format!(
        "Create snapshot {snapshot_uuid} of virtual machine with UUID {}",
        virtual_machine_data.uuid
    );
    let info = CloudHypervisorVirtualMachineSnapshotInfo {
        vm_uuid: virtual_machine_data.uuid,
        snapshot_uuid,
        name: task_name.clone(),
        context: context.clone(),
    };

    // create new task, which is processed by the same worker-thread as all other tasks of the
    // virtual_machine, so the snapshot can not overtake its creation
    let task = Task {
        uuid: task_uuid,
        resouce_uuid: *virtual_machine_uuid,
        resource_type: TaskResourceType::VirtualMachine,
        name: task_name,
        info: TaskVariant::CloudHypervisorVirtualMachineSnapshot(info),
        meta: TaskMeta::new(),
    };
    super::super::task::add_task(task, &task_type, &context)
        .inspect_err(|e| log::error!("Creating a snapshot-task failed with error: {e}"))?;

    // get new created task from database to get additional information
    let task_data = task_table::get_task(&task_uuid, &context)
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
