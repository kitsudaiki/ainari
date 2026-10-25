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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::backup_table::{self, StatusChange};
use crate::database::host_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::enums;

#[api_operation(
    tag = "backup",
    summary = "Delete backup",
    description = r###"Delete a backup together with its whole package on the backup-target.

The package is only deleted as a whole and can be deleted in every state. It gets the state
`DELETING` first, so it can't be read anymore. If the deletion of the files fails, the backup stays
in this state and the deletion can be repeated.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn delete_backup_internal(
    backup_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let backup = super::get_backup_entry(&backup_uuid, &context)?;

    let changed = backup_table::change_backup_status(
        &backup_uuid,
        &[
            BackupStatus::Creating,
            BackupStatus::Ready,
            BackupStatus::Error,
            BackupStatus::Deleting,
        ],
        BackupStatus::Deleting,
        None,
        &context,
    )
    .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;
    if changed == StatusChange::WrongStatus {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' was changed in the meantime.",
            *backup_uuid
        )));
    }

    match host_table::get_host(&backup.host_uuid, &context) {
        Ok(host) => {
            onsen_file_transfer::delete_directory(&host.address, &backup.directory_path)
                .await
                .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
        }
        // the files are gone together with the host, so only the database-entry is left
        Err(enums::DbError::NotFound) => {
            log::warn!(
                "Onsen-host '{}' of backup '{}' doesn't exist anymore, so only the backup is \
                 removed from the database",
                backup.host_uuid,
                backup.uuid
            );
        }
        Err(e) => {
            return Err(map_db_uuid_get_delete_error(
                "onsen-host",
                &backup.host_uuid,
                e,
                Some(&context),
            ));
        }
    }

    let changed = backup_table::delete_backup(&backup_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;
    if changed == StatusChange::WrongStatus {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' was changed in the meantime.",
            *backup_uuid
        )));
    }

    Ok(NoContent)
}
