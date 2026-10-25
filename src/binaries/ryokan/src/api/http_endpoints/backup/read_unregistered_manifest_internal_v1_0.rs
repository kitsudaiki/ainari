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

use actix_web::web::{Json, Path, Query};
use apistos::api_operation;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use uuid::Uuid;

use crate::database::backup_table;
use crate::database::host_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "backup",
    summary = "Read manifest of unregistered package",
    description = r###"Read the manifest of a package on a backup-target, which is not registered as
backup yet, like a package, which was copied from another deployment.

The manifest describes, to which project the package belongs, so it is read before the package is
registered. Only packages, which are found by the scan of the backup-target, can be read. The
manifest is returned base64-encoded. This can only be done by an admin.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn read_unregistered_manifest_internal(
    host_uuid: Path<Uuid>,
    query: Query<BackupManifestQuery>,
    context: UserContext,
) -> Result<Json<BackupFileResp>, ErrorResponse> {
    check_admin_context(&context)?;

    let host = host_table::get_host(&host_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("onsen-host", &host_uuid, e, Some(&context)))?;
    if !host.is_backup_target {
        return Err(ErrorResponse::Conflict(format!(
            "Onsen-host with UUID '{}' is no backup-target.",
            *host_uuid
        )));
    }
    // a registered package is only read over the backup itself, which checks its state
    if backup_table::is_registered(&query.backup_uuid)
        .map_err(|e| map_db_list_error("backups", e, Some(&context)))?
    {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' is already registered.",
            query.backup_uuid
        )));
    }

    let manifest_path = format!(
        "{}/{}",
        backup_table::backup_directory_path(&query.backup_uuid),
        super::MANIFEST_FILE
    );
    let content =
        onsen_file_transfer::download_data(&host.address, &manifest_path, MAX_BACKUP_FILE_SIZE)
            .await
            .map_err(|e| match e {
                AinariError::NotFound(_) => ErrorResponse::NotFound(format!(
                    "Package '{}' on onsen-host '{}' has no '{}'.",
                    query.backup_uuid,
                    *host_uuid,
                    super::MANIFEST_FILE
                )),
                e => map_ainari_error_to_api_response(e, Some(&context)),
            })?;

    Ok(Json(BackupFileResp {
        path: super::MANIFEST_FILE.to_string(),
        content: BASE64.encode(content),
    }))
}
