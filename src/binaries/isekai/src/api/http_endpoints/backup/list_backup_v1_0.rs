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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_to_backup_resp() {
        let entry = BackupEntry {
            uuid: Uuid::new_v4(),
            name: "backup".to_string(),
            backup_kind: "PROJECT".to_string(),
            project_id: "project".to_string(),
            virtual_machine_uuid: None,
            backup_state: "CREATING".to_string(),
            ainari_version: "0.22.0".to_string(),
            secret_uuids: Vec::new(),
            owner_id: "user".to_string(),
            created_at: Utc::now(),
            created_by: "user".to_string(),
            updated_at: Utc::now(),
            updated_by: "user".to_string(),
        };
        let mut package = BackupResp {
            uuid: entry.uuid,
            host_uuid: Uuid::new_v4(),
            onsen_address: "http://onsen:50051".to_string(),
            directory_path: format!("backups/{}", entry.uuid),
            backup_status: BackupStatus::Creating,
            file_count: 0,
            total_size: 0,
            manifest_sha256: None,
            owner_id: "user".to_string(),
            project_id: "project".to_string(),
            created_at: Utc::now(),
            created_by: "user".to_string(),
            updated_at: Utc::now(),
            updated_by: "user".to_string(),
        };

        // the size is unknown, until the package is sealed
        let resp = to_backup_resp(entry.clone(), Some(&package)).unwrap();
        assert_eq!(resp.kind, BackupKind::Project);
        assert_eq!(resp.state, BackupStatus::Creating);
        assert_eq!(resp.host_uuid, Some(package.host_uuid));
        assert!(resp.total_size.is_none());

        package.backup_status = BackupStatus::Ready;
        package.total_size = 42;
        package.file_count = 3;
        let resp = to_backup_resp(entry.clone(), Some(&package)).unwrap();
        assert_eq!(resp.total_size, Some(42));
        assert_eq!(resp.file_count, Some(3));

        let resp = to_backup_resp(entry.clone(), None).unwrap();
        assert!(resp.host_uuid.is_none());

        let invalid = BackupEntry {
            backup_kind: "UNKNOWN".to_string(),
            ..entry
        };
        assert!(to_backup_resp(invalid, None).is_err());
    }
}
