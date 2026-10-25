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

use crate::core::processing::tasks::{
    CloudHypervisorVirtualMachineSnapshotInfo, SnapshotKind, Task, TaskMeta, TaskVariant,
};
use crate::database::task_table;
use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "task",
    summary = "Create new backup-task (internal)",
    description = r###"Create a new task, which saves the root-disk of a virtual_machine into a backup.

It works like a snapshot, but the root-disk is no image: it is encrypted with the secret
`secret_uuid` and uploaded into the package of the backup `backup_uuid`, which must be in the
state `CREATING`. The task is processed by the same worker-thread as all other tasks of the
virtual_machine, so it can't overtake them.

By default the virtual_machine is only paused, while its root-disk is copied, so the saved
root-disk is only crash-consistent. With `stop_vm` a running virtual_machine is shut down before
and started again afterwards, so the saved root-disk is consistent.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn backup_virtual_machine_internal(
    body: Json<TaskBackupSaveReq>,
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let task_uuid = Uuid::new_v4();
    let task_type = TaskType::SnapshotSave;

    let virtual_machine_data = virtual_machine_table::get_virtual_machine(
        &virtual_machine_uuid,
        &context,
    )
    .map_err(|e| {
        map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e, Some(&context))
    })?;
    super::reject_migrating(&virtual_machine_data)?;

    let task_description = format!(
        "Save root-disk of virtual machine with UUID {} into backup {}",
        virtual_machine_data.uuid, body.backup_uuid
    );
    let info = CloudHypervisorVirtualMachineSnapshotInfo {
        vm_uuid: virtual_machine_data.uuid,
        kind: SnapshotKind::BackupDisk {
            backup_uuid: body.backup_uuid,
        },
        secret_uuid: body.secret_uuid,
        stop_vm: body.stop_vm,
        description: task_description.clone(),
        context: context.clone(),
    };

    let task = Task {
        uuid: task_uuid,
        resouce_uuid: *virtual_machine_uuid,
        resource_type: TaskResourceType::VirtualMachine,
        description: task_description,
        info: TaskVariant::CloudHypervisorVirtualMachineSnapshot(info),
        meta: TaskMeta::new(),
    };
    super::super::task::add_task(task, &task_type, &context)
        .inspect_err(|e| log::error!("Creating a backup-task failed with error: {e}"))?;

    let task_data = task_table::get_task(&task_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("task", &task_uuid, e, Some(&context)))?;

    Ok(CreatedJson(TaskResp {
        uuid: task_uuid,
        description: task_data.description,
        task_type: task_data.task_type,
        state: task_data.task_state,
        queued_at: task_data.queued_at,
        started_at: task_data.started_at,
        finished_at: task_data.finished_at,
        messages: task_data.messages,
        created_by: task_data.created_by,
    }))
}
