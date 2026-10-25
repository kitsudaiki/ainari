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

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::backup as backup_clients;

use crate::core::services::Services;
use crate::database::backup_table;

#[api_operation(
    tag = "backup",
    summary = "Get backup",
    description = r###"Get a backup of the project of the user.

The backup-target and the size of the backup are read from ryokan, which stores the files of the
backup. They are empty, if ryokan doesn't know the backup or is not reachable."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_backup(
    backup_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<IsekaiBackupResp>, ErrorResponse> {
    let entry = backup_table::get_backup(&backup_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;

    let package = match Services::connect(&context).await {
        Ok(services) => backup_clients::get_backup(
            &services.endpoints.ryokan,
            &services.token,
            services.key(),
            &backup_uuid,
            services.insecure(),
        )
        .await
        .inspect_err(|e| log::warn!("Failed to get backup {} from ryokan: {e}", *backup_uuid))
        .ok(),
        Err(e) => {
            log::warn!("Failed to get the endpoints from miko: {e}");
            None
        }
    };

    let kind = entry
        .kind()
        .map_err(|e| map_internal_error("read kind of backup", e, None))?;
    let state = entry
        .state()
        .map_err(|e| map_internal_error("read state of backup", e, None))?;
    // the size is only known, after the package was sealed
    let sealed = package
        .as_ref()
        .filter(|package| package.backup_status == BackupStatus::Ready);

    let resp = IsekaiBackupResp {
        uuid: entry.uuid,
        name: entry.name,
        kind,
        project_id: entry.project_id,
        virtual_machine_uuid: entry.virtual_machine_uuid,
        state,
        ainari_version: entry.ainari_version,
        host_uuid: package.as_ref().map(|package| package.host_uuid),
        file_count: sealed.map(|package| package.file_count),
        total_size: sealed.map(|package| package.total_size),
        created_at: entry.created_at,
        created_by: entry.created_by,
    };

    Ok(Json(resp))
}
