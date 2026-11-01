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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::database::backup_table::{self, BackupAddResult};
use crate::database::host_table;

use ainari_api::common_functions::*;
use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "backup",
    summary = "Register backup",
    description = r###"Register a package, which was copied onto a backup-target from outside, for
example from another deployment, as backup.

The package must be in `backups/<uuid>` on the backup-target and contain a manifest. Like with
finalizing a backup, the number and size of all files and the sha256 of the manifest are read from
the backup-target, and the backup is directly registered as sealed package in the state `READY`.
This can only be done by an admin.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn register_backup_internal(
    body: Json<BackupRegisterReq>,
    context: UserContext,
) -> Result<CreatedJson<BackupResp>, ErrorResponse> {
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let host = host_table::get_host(&body.host_uuid, &context).map_err(|e| {
        map_db_uuid_get_delete_error("onsen-host", &body.host_uuid, e, Some(&context))
    })?;
    if backup_table::is_registered(&body.uuid)
        .map_err(|e| map_db_list_error("backups", e, Some(&context)))?
    {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' already exists.",
            body.uuid
        )));
    }

    let package_path = backup_table::backup_directory_path(&body.uuid);
    let Some(package) = super::read_package_info(&host.address, &package_path, &context).await?
    else {
        return Err(ErrorResponse::Conflict(format!(
            "Package '{package_path}' on onsen-host '{}' doesn't exist or has no '{}'.",
            body.host_uuid,
            super::MANIFEST_FILE
        )));
    };

    let result = backup_table::add_new_backup(
        &body.uuid,
        &body.host_uuid,
        &body.project_id,
        Some(&package),
        &context,
    )
    .map_err(|e| map_db_write_error("add backup to database", e, Some(&context)))?;
    match result {
        BackupAddResult::Added => {}
        BackupAddResult::NoBackupTarget => {
            return Err(ErrorResponse::Conflict(format!(
                "Onsen-host with UUID '{}' is no backup-target.",
                body.host_uuid
            )));
        }
        BackupAddResult::AlreadyExists => {
            return Err(ErrorResponse::Conflict(format!(
                "Backup with UUID '{}' already exists.",
                body.uuid
            )));
        }
    }

    let backup = backup_table::get_backup(&body.uuid, &context)
        .map_err(|e| map_db_uuid_get_after_add_error("backup", &body.uuid, e, Some(&context)))?;

    Ok(CreatedJson(super::to_backup_resp(backup, &context)?))
}
