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

use actix_web::web::{Json, Query};
use apistos::api_operation;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::backup::scan_backup_target;

use crate::core::services::Services;
use crate::database::backup_table;

#[api_operation(
    tag = "import",
    summary = "Scan backup-target for foreign backups",
    description = r###"List the backups on a backup-target, which are not known yet.

These are backups, which were copied onto the backup-target from outside, for example from
another deployment for a disaster-recovery. Only complete backups with a manifest are listed. They
can be imported with `POST /v1alpha/backup/import/admin`. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn scan_backup_target_admin(
    query: Query<BackupScanQuery>,
    context: UserContext,
) -> Result<Json<BackupScanResp>, ErrorResponse> {
    check_admin_context(&context)?;

    let services = Services::connect(&context)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    let scan = scan_backup_target(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &query.onsen_host_uuid,
        services.insecure(),
    )
    .await
    .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    let mut backups = Vec::new();
    for entry in scan.backups.into_iter().filter(|entry| entry.has_manifest) {
        let known = backup_table::is_known(&entry.uuid)
            .map_err(|e| map_db_list_error("backups", e, Some(&context)))?;
        if !known {
            backups.push(entry);
        }
    }

    Ok(Json(BackupScanResp { backups }))
}
