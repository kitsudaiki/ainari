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
use std::collections::HashMap;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::backup as backup_clients;

use crate::core::services::Services;
use crate::database::backup_table::{self, BackupEntry};

#[api_operation(
    tag = "backup",
    summary = "List backups",
    description = r###"List all backups of the project of the user, the newest first.

Admins see the backups of all projects, which can be restricted to one project with the
query-parameter `project_id`."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_backup(
    query: Query<BackupListQuery>,
    context: UserContext,
) -> Result<Json<IsekaiBackupListResp>, ErrorResponse> {
    let entries = backup_table::list_backups(&context, query.project_id.as_deref())
        .map_err(|e| map_db_list_error("backups", e, Some(&context)))?;

    let packages: HashMap<_, _> = match Services::connect(&context).await {
        Ok(services) => backup_clients::list_backups(
            &services.endpoints.ryokan,
            &services.token,
            services.key(),
            query.project_id.as_deref(),
            services.insecure(),
        )
        .await
        .map(|list| list.backups)
        .inspect_err(|e| log::warn!("Failed to list the backups of ryokan: {e}"))
        .unwrap_or_default(),
        Err(e) => {
            log::warn!("Failed to get the endpoints from miko: {e}");
            Vec::new()
        }
    }
    .into_iter()
    .map(|package| (package.uuid, package))
    .collect();

    let backups = entries
        .into_iter()
        .map(|entry| {
            let package = packages.get(&entry.uuid);
            to_backup_resp(entry, package)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(IsekaiBackupListResp { backups }))
}

/// Converts a backup of the database into its response.
///
/// # Arguments
///
/// * `entry` - The backup
/// * `package` - The backup-resource in ryokan with the same UUID, if it is known
///
/// # Returns
///
/// The response, or an internal error, if the database contains invalid values.
fn to_backup_resp(
    entry: BackupEntry,
    package: Option<&BackupResp>,
) -> Result<IsekaiBackupResp, ErrorResponse> {
    let kind = entry
        .kind()
        .map_err(|e| map_internal_error("read kind of backup", e, None))?;
    let state = entry
        .state()
        .map_err(|e| map_internal_error("read state of backup", e, None))?;
    // the size is only known, after the package was sealed
    let sealed = package.filter(|package| package.backup_status == BackupStatus::Ready);

    Ok(IsekaiBackupResp {
        uuid: entry.uuid,
        name: entry.name,
        kind,
        project_id: entry.project_id,
        virtual_machine_uuid: entry.virtual_machine_uuid,
        state,
        ainari_version: entry.ainari_version,
        host_uuid: package.map(|package| package.host_uuid),
        file_count: sealed.map(|package| package.file_count),
        total_size: sealed.map(|package| package.total_size),
        created_at: entry.created_at,
        created_by: entry.created_by,
    })
}
