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

pub mod copy_backup_file_internal_v1_0;
pub mod create_backup_internal_v1_0;
pub mod delete_backup_internal_v1_0;
pub mod finalize_backup_internal_v1_0;
pub mod get_backup_file_info_internal_v1_0;
pub mod get_backup_internal_v1_0;
pub mod list_backup_internal_v1_0;
pub mod prepare_backup_disk_internal_v1_0;
pub mod read_backup_file_internal_v1_0;
pub mod read_unregistered_manifest_internal_v1_0;
pub mod register_backup_internal_v1_0;
pub mod scan_backup_target_internal_v1_0;
pub mod write_backup_file_internal_v1_0;

use std::path::{Component, Path};
use uuid::Uuid;

use crate::database::backup_table::{self, BackupEntry, PackageInfo};
use crate::database::host_table::{self, HostEntry};
use crate::onsen_functions::{RemoteFile, list_files_recursive};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::enums;
use ainari_common::error::AinariError;

/// Name of the manifest within the package. A package without manifest is incomplete.
pub const MANIFEST_FILE: &str = "manifest.json";

/// Directory within the package, which contains the root-disks of the virtual_machines.
pub const DISK_DIRECTORY: &str = "disks";

/// Checks, that a path of a file within a package stays within the directory of the package.
///
/// # Arguments
///
/// * `path` - Path of the file relative to the directory of the package
///
/// # Returns
///
/// * `Ok(())` - The path is valid
/// * `Err(ErrorResponse::BadRequest)` - The path is empty, absolute or contains `.` or `..`
fn check_package_path(path: &str) -> Result<(), ErrorResponse> {
    let is_valid = !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if !is_valid {
        return Err(ErrorResponse::BadRequest(format!(
            "'{path}' is no valid path of a file within a backup."
        )));
    }
    Ok(())
}

/// Returns the backup from the database, which is accessible for the user.
///
/// # Arguments
///
/// * `backup_uuid` - UUID of the backup
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(BackupEntry)` - The backup
/// * `Err(ErrorResponse::NotFound)` - The backup doesn't exist or is not accessible
fn get_backup_entry(
    backup_uuid: &Uuid,
    context: &UserContext,
) -> Result<BackupEntry, ErrorResponse> {
    backup_table::get_backup(backup_uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("backup", backup_uuid, e, Some(context)))
}

/// Returns the state of a backup from the database as value of the api.
///
/// # Arguments
///
/// * `backup` - The backup from the database
///
/// # Returns
///
/// * `Ok(BackupStatus)` - The state of the package
/// * `Err(ErrorResponse::InternalError)` - If the database contains an unknown state
fn backup_status_of(backup: &BackupEntry) -> Result<BackupStatus, ErrorResponse> {
    backup.backup_status.parse().map_err(|e| {
        ainari_api::report_internal_error!(
            None,
            "Invalid status of backup '{}' in database: {e}",
            backup.uuid
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })
}

/// Checks, that a package is in the expected state, before its files are accessed.
///
/// # Arguments
///
/// * `backup` - The backup from the database
/// * `expected` - The state, which is required for the operation
/// * `operation` - Description of the operation for the error-message
///
/// # Returns
///
/// * `Ok(())` - The package is in the expected state
/// * `Err(ErrorResponse::Conflict)` - The package is in another state
fn check_backup_status(
    backup: &BackupEntry,
    expected: BackupStatus,
    operation: &str,
) -> Result<(), ErrorResponse> {
    let current = backup_status_of(backup)?;
    if current != expected {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' is in state '{current}', but {operation} requires the state \
             '{expected}'.",
            backup.uuid
        )));
    }
    Ok(())
}

/// Returns the backup-target, which stores the package of a backup.
///
/// # Arguments
///
/// * `backup` - The backup from the database
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(HostEntry)` - The backup-target
/// * `Err(ErrorResponse::InternalError)` - The host doesn't exist anymore or the database failed
fn get_backup_host(
    backup: &BackupEntry,
    context: &UserContext,
) -> Result<HostEntry, ErrorResponse> {
    host_table::get_host(&backup.host_uuid, context).map_err(|e| match e {
        enums::DbError::NotFound => {
            ainari_api::report_internal_error!(
                Some(context),
                "Onsen-host '{}' of backup '{}' doesn't exist anymore",
                backup.host_uuid,
                backup.uuid
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        }
        e => map_db_uuid_get_delete_error("onsen-host", &backup.host_uuid, e, Some(context)),
    })
}

/// Converts a backup from the database into the response of the api.
///
/// # Arguments
///
/// * `backup` - The backup from the database
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(BackupResp)` - The response. The onsen-address is empty, if the backup-target was
///   deleted in the meantime.
/// * `Err(ErrorResponse::InternalError)` - The database failed or contains invalid values
fn to_backup_resp(backup: BackupEntry, context: &UserContext) -> Result<BackupResp, ErrorResponse> {
    let onsen_address = match host_table::get_host(&backup.host_uuid, context) {
        Ok(host) => host.address,
        Err(enums::DbError::NotFound) => String::new(),
        Err(e) => {
            return Err(map_db_uuid_get_delete_error(
                "onsen-host",
                &backup.host_uuid,
                e,
                Some(context),
            ));
        }
    };

    Ok(BackupResp {
        uuid: backup.uuid,
        host_uuid: backup.host_uuid,
        onsen_address,
        directory_path: backup.directory_path.clone(),
        backup_status: backup_status_of(&backup)?,
        file_count: backup.file_count.max(0) as u64,
        total_size: backup.total_size.max(0) as u64,
        manifest_sha256: backup.manifest_sha256,
        owner_id: backup.owner_id,
        project_id: backup.project_id,
        created_at: backup.created_at,
        created_by: backup.created_by,
        updated_at: backup.updated_at,
        updated_by: backup.updated_by,
    })
}

/// Reads the summary of the files of a package from the onsen.
///
/// # Arguments
///
/// * `onsen_address` - Address of the backup-target
/// * `directory_path` - Path of the package on the onsen
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(Some(PackageInfo))` - Number and size of all files together with the sha256 of the
///   manifest
/// * `Ok(None)` - The package has no manifest, so it is incomplete
/// * `Err(ErrorResponse::InternalError)` - The onsen could not be asked
async fn read_package_info(
    onsen_address: &str,
    directory_path: &str,
    context: &UserContext,
) -> Result<Option<PackageInfo>, ErrorResponse> {
    let files = match list_files_recursive(onsen_address, directory_path).await {
        Ok(files) => files,
        // nothing was written into the package yet
        Err(AinariError::NotFound(_)) => Vec::new(),
        Err(e) => return Err(map_ainari_error_to_api_response(e, Some(context))),
    };
    if !has_manifest(&files) {
        return Ok(None);
    }

    let manifest_path = format!("{directory_path}/{MANIFEST_FILE}");
    let manifest_info = onsen_file_transfer::get_file_info(onsen_address, &manifest_path)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;

    let (file_count, total_size) = summarize(&files).ok_or_else(|| {
        ainari_api::report_internal_error!(
            Some(context),
            "Package '{directory_path}' on onsen '{onsen_address}' is too large to be registered"
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(Some(PackageInfo {
        file_count,
        total_size,
        manifest_sha256: manifest_info.sha256,
    }))
}

/// Checks, if the manifest is part of the files of a package.
fn has_manifest(files: &[RemoteFile]) -> bool {
    files.iter().any(|file| file.path == MANIFEST_FILE)
}

/// Returns number and total size of the files of a package, or None, if they don't fit into the
/// columns of the database.
fn summarize(files: &[RemoteFile]) -> Option<(i32, i64)> {
    let file_count = i32::try_from(files.len()).ok()?;
    let total_size = files
        .iter()
        .try_fold(0u64, |sum, file| sum.checked_add(file.size))?;
    Some((file_count, i64::try_from(total_size).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, size: u64) -> RemoteFile {
        RemoteFile {
            path: path.to_string(),
            size,
        }
    }

    #[test]
    fn test_check_package_path() {
        for path in ["manifest.json", "metadata/miko.json.enc", "disks/vm.enc"] {
            assert!(check_package_path(path).is_ok(), "{path}");
        }
        for path in [
            "",
            "/etc/passwd",
            "../other",
            "metadata/../../x",
            ".",
            "./x",
        ] {
            assert!(check_package_path(path).is_err(), "{path}");
        }
    }

    #[test]
    fn test_has_manifest() {
        assert!(has_manifest(&[
            file("disks/vm.enc", 10),
            file("manifest.json", 1)
        ]));
        // only the manifest in the root of the package counts
        assert!(!has_manifest(&[file("metadata/manifest.json", 1)]));
        assert!(!has_manifest(&[]));
    }

    #[test]
    fn test_summarize() {
        assert_eq!(summarize(&[]), Some((0, 0)));
        assert_eq!(
            summarize(&[file("manifest.json", 100), file("disks/vm.enc", 1 << 40)]),
            Some((2, 100 + (1 << 40)))
        );
        assert_eq!(summarize(&[file("a", u64::MAX), file("b", 1)]), None);
        assert_eq!(summarize(&[file("a", u64::MAX)]), None);
    }
}
