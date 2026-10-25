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

use crate::database::backup_table::{self, StatusChange};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "backup",
    summary = "Finalize backup",
    description = r###"Seal the package of a backup, after all files were written into it.

The number and size of all files and the sha256 of the manifest are read from the backup-target
and stored together with the state `READY`. From now on the package is read only. A package
without `manifest.json` is incomplete, so it gets the state `ERROR` and can only be deleted.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn finalize_backup_internal(
    backup_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<BackupResp>, ErrorResponse> {
    let backup = super::get_backup_entry(&backup_uuid, &context)?;
    super::check_backup_status(&backup, BackupStatus::Creating, "finalizing")?;
    let host = super::get_backup_host(&backup, &context)?;

    let package = super::read_package_info(&host.address, &backup.directory_path, &context).await?;

    let (new_status, package) = match &package {
        Some(package) => (BackupStatus::Ready, Some(package)),
        None => (BackupStatus::Error, None),
    };
    let changed = backup_table::change_backup_status(
        &backup_uuid,
        &[BackupStatus::Creating],
        new_status,
        package,
        &context,
    )
    .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;
    if changed == StatusChange::WrongStatus {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' was changed in the meantime.",
            *backup_uuid
        )));
    }
    if package.is_none() {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' has no '{}', so it is incomplete and was set to 'ERROR'.",
            *backup_uuid,
            super::MANIFEST_FILE
        )));
    }

    let backup = super::get_backup_entry(&backup_uuid, &context)?;
    Ok(Json(super::to_backup_resp(backup, &context)?))
}
