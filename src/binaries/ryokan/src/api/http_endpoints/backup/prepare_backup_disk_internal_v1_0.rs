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

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "backup",
    summary = "Prepare disk in backup",
    description = r###"Get the target, where the root-disk of a virtual_machine is uploaded into the
package of a backup.

The disk is uploaded directly into the onsen of the backup-target, so only its address and the path
of the disk are returned. The disk is no image, so it is not registered as image. This is only
possible, while the backup is in the state `CREATING`.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn prepare_backup_disk_internal(
    backup_uuid: Path<Uuid>,
    body: Json<BackupDiskReq>,
    context: UserContext,
) -> Result<Json<BackupDiskResp>, ErrorResponse> {
    if context.is_read_only() {
        return Err(permission_denied_response());
    }

    let backup = super::get_backup_entry(&backup_uuid, &context)?;
    super::check_backup_status(&backup, BackupStatus::Creating, "adding a disk")?;
    let host = super::get_backup_host(&backup, &context)?;

    Ok(Json(BackupDiskResp {
        onsen_address: host.address,
        file_path: format!(
            "{}/{}/{}.enc",
            backup.directory_path,
            super::DISK_DIRECTORY,
            body.vm_uuid
        ),
    }))
}
