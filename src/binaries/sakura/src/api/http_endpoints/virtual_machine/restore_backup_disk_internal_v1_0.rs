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

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "task",
    summary = "Create new backup-restore-task (internal)",
    description = r###"Create a new task, which replaces the root-disk of an existing virtual_machine
with the root-disk of a backup.

The root-disk of the backup `backup_uuid` was copied out of the backup and registered as
snapshot-image `image_uuid` before. The task works like the restore of a snapshot, but has its own
type, so the tasks of the virtual_machine show, that it was reset to a backup. The
virtual_machine is shut down, while its root-disk is replaced, and booted again afterwards.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn restore_backup_disk_internal(
    body: Json<TaskBackupRestoreReq>,
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<CreatedJson<TaskResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let backup_uuid = body.backup_uuid;
    let resp = super::snapshot_restore_v1_0::create_restore_task(
        &virtual_machine_uuid,
        &body.image_uuid,
        TaskType::BackupRestore,
        |_, virtual_machine_uuid| {
            format!(
                "Restore root-disk of virtual machine with UUID {virtual_machine_uuid} from \
                 backup {backup_uuid}"
            )
        },
        &context,
    )
    .await?;
    Ok(CreatedJson(resp))
}
