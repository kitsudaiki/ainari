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

use crate::database::backup_table;
use crate::database::host_table;
use crate::onsen_functions::BACKUP_DIRECTORY;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_clients::onsen_file_transfer::data::DirectoryEntry;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "backup",
    summary = "Scan backup-target",
    description = r###"List the packages on a backup-target, which are not registered as backup.

These are packages, which were copied onto the backup-target from outside, for example from another
deployment. Only directories in `backups/`, whose name is a UUID, are listed. `has_manifest` shows,
if the package is complete and can be registered. This can only be done by an admin.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn scan_backup_target_internal(
    host_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<BackupScanResp>, ErrorResponse> {
    check_admin_context(&context)?;

    let host = host_table::get_host(&host_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("onsen-host", &host_uuid, e, Some(&context)))?;
    if !host.is_backup_target {
        return Err(ErrorResponse::Conflict(format!(
            "Onsen-host with UUID '{}' is no backup-target.",
            *host_uuid
        )));
    }

    let entries = list_or_empty(&host.address, BACKUP_DIRECTORY)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    let mut backups = Vec::new();
    for backup_uuid in package_uuids(&entries) {
        let registered = backup_table::is_registered(&backup_uuid)
            .map_err(|e| map_db_list_error("backups", e, Some(&context)))?;
        if registered {
            continue;
        }

        let package_path = backup_table::backup_directory_path(&backup_uuid);
        let files = list_or_empty(&host.address, &package_path)
            .await
            .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
        let has_manifest = files
            .iter()
            .any(|file| !file.is_dir && file.name == super::MANIFEST_FILE);
        backups.push(BackupScanEntry {
            uuid: backup_uuid,
            has_manifest,
        });
    }

    Ok(Json(BackupScanResp { backups }))
}

/// Lists a directory of an onsen, where a not existing directory counts as empty.
async fn list_or_empty(
    onsen_address: &str,
    dir_path: &str,
) -> Result<Vec<DirectoryEntry>, AinariError> {
    match onsen_file_transfer::list_directory(onsen_address, dir_path).await {
        Ok(entries) => Ok(entries),
        Err(AinariError::NotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Returns the UUIDs of the packages within the backup-directory. Files and directories, whose
/// name is no UUID, are no packages and are skipped.
fn package_uuids(entries: &[DirectoryEntry]) -> Vec<Uuid> {
    entries
        .iter()
        .filter(|entry| entry.is_dir)
        .filter_map(|entry| {
            // only the canonical form is the name of a package, which ryokan can address again
            Uuid::parse_str(&entry.name)
                .ok()
                .filter(|backup_uuid| backup_uuid.to_string() == entry.name)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> DirectoryEntry {
        DirectoryEntry {
            name: name.to_string(),
            is_dir,
            size: 0,
        }
    }

    #[test]
    fn test_package_uuids() {
        let package = Uuid::new_v4();
        let file = Uuid::new_v4();
        let entries = vec![
            entry(&package.to_string(), true),
            entry(&file.to_string(), false),
            entry("lost+found", true),
            entry(&package.simple().to_string(), true),
            entry(&package.to_string().to_uppercase(), true),
        ];
        assert_eq!(package_uuids(&entries), vec![package]);
        assert!(package_uuids(&[]).is_empty());
    }
}
