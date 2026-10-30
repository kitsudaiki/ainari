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

use rand::prelude::IndexedRandom;

use crate::database::host_table;
use crate::database::host_table::HostEntry;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_clients::onsen_file_transfer::data::DirectoryEntry;
use ainari_common::error::AinariError;

/// Directory on a backup-target, which contains the backups.
pub const BACKUP_DIRECTORY: &str = "backups";

/// Selects the onsen-host, on which new files are stored.
///
/// The host is picked randomly out of all registered hosts, which are no backup-targets. This is
/// not a real scheduling yet, so neither the free space nor the load of the hosts is taken into
/// account.
///
/// # Arguments
///
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(HostEntry)` - The selected onsen-host.
/// * `Err(ErrorResponse::InternalError)` - The hosts could not be read or no host for images is
///   registered.
pub fn select_onsen(context: &UserContext) -> Result<HostEntry, ErrorResponse> {
    // list all available hosts
    let hosts = match host_table::list_hosts(context) {
        Ok(hosts) => hosts,
        Err(e) => {
            ainari_api::report_internal_error!(
                Some(context),
                "Failed to get list of hosts form database: '{e}'"
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // pick one of the hosts at random
    let Some(selected_host) = pick_image_host(&hosts) else {
        ainari_api::report_internal_error!(
            Some(context),
            "No onsen-host, which is not a backup-target, to store new images."
        );
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    };

    Ok(selected_host)
}

/// Selects the backup-target, on which a new backup is stored.
///
/// Like for images, the host is picked randomly out of all registered backup-targets, so neither
/// the free space nor the load of the hosts is taken into account yet.
///
/// # Arguments
///
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(HostEntry)` - The selected backup-target.
/// * `Err(ErrorResponse::Conflict)` - No backup-target is registered.
/// * `Err(ErrorResponse::InternalError)` - The hosts could not be read.
pub fn select_backup_target(context: &UserContext) -> Result<HostEntry, ErrorResponse> {
    let hosts = host_table::list_hosts(context).map_err(|e| {
        ainari_api::report_internal_error!(
            Some(context),
            "Failed to get list of hosts form database: '{e}'"
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    pick_backup_host(&hosts).ok_or_else(|| {
        ErrorResponse::Conflict("No onsen-host is marked as backup-target.".to_string())
    })
}

/// A file within a directory of an onsen.
#[derive(Debug, PartialEq, Clone)]
pub struct RemoteFile {
    /// Path of the file relative to the listed directory
    pub path: String,
    /// Size of the file in bytes
    pub size: u64,
}

/// Lists all files within a directory of an onsen, including the files of its subdirectories.
///
/// # Arguments
///
/// * `onsen_address` - Address of the onsen-host
/// * `dir_path` - Path of the directory relative to the storage-location of the onsen
///
/// # Returns
///
/// * `Ok(Vec<RemoteFile>)` - All files with their path relative to `dir_path`, sorted by path.
/// * `Err(AinariError::NotFound)` - The directory doesn't exist.
/// * `Err(AinariError)` - The onsen could not be asked.
pub async fn list_files_recursive(
    onsen_address: &str,
    dir_path: &str,
) -> Result<Vec<RemoteFile>, AinariError> {
    let mut files = Vec::new();
    // relative paths of the directories, which still have to be listed
    let mut pending = vec![String::new()];

    while let Some(relative_dir) = pending.pop() {
        let full_path = join_path(dir_path, &relative_dir);
        for entry in onsen_file_transfer::list_directory(onsen_address, &full_path).await? {
            let relative_path = join_path(&relative_dir, &entry.name);
            if entry.is_dir {
                pending.push(relative_path);
            } else {
                files.push(RemoteFile {
                    path: relative_path,
                    size: entry.size,
                });
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(files)
}

/// Joins two parts of a relative path, where the first one can be empty.
fn join_path(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_owned()
    } else {
        format!("{base}/{name}")
    }
}

/// Checks, if backups are stored on an onsen-host.
///
/// # Arguments
///
/// * `onsen_address` - Address of the onsen-host
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(bool)` - True, if the backup-directory of the host isn't empty.
/// * `Err(ErrorResponse::InternalError)` - The onsen could not be asked.
pub async fn has_backups(
    onsen_address: &str,
    context: &UserContext,
) -> Result<bool, ErrorResponse> {
    let listing = onsen_file_transfer::list_directory(onsen_address, BACKUP_DIRECTORY).await;
    is_listing_not_empty(listing).map_err(|e| {
        ainari_api::report_internal_error!(
            Some(context),
            "Failed to list backups on onsen '{onsen_address}': {e}"
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })
}

/// Checks, if a directory-listing of the onsen contains any entries. A not existing directory
/// counts as empty.
fn is_listing_not_empty(
    listing: Result<Vec<DirectoryEntry>, AinariError>,
) -> Result<bool, AinariError> {
    match listing {
        Ok(entries) => Ok(!entries.is_empty()),
        Err(AinariError::NotFound(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Picks a random host out of the given hosts, which is no backup-target.
///
/// # Arguments
///
/// * `hosts` - The available hosts
///
/// # Returns
///
/// The picked host, or None, if there is no host for images.
fn pick_image_host(hosts: &[HostEntry]) -> Option<HostEntry> {
    let image_hosts: Vec<&HostEntry> = hosts.iter().filter(|h| !h.is_backup_target).collect();
    image_hosts
        .choose(&mut rand::rng())
        .map(|host| (*host).clone())
}

/// Picks a random host out of the given hosts, which is a backup-target.
///
/// # Arguments
///
/// * `hosts` - The available hosts
///
/// # Returns
///
/// The picked host, or None, if there is no backup-target.
fn pick_backup_host(hosts: &[HostEntry]) -> Option<HostEntry> {
    let backup_hosts: Vec<&HostEntry> = hosts.iter().filter(|h| h.is_backup_target).collect();
    backup_hosts
        .choose(&mut rand::rng())
        .map(|host| (*host).clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn host(is_backup_target: bool) -> HostEntry {
        HostEntry {
            uuid: Uuid::new_v4(),
            name: "onsen".to_string(),
            address: format!("http://{}:50051", Uuid::new_v4()),
            is_backup_target,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        }
    }

    #[test]
    fn test_pick_image_host_skips_backup_targets() {
        let image_host = host(false);
        let hosts = vec![host(true), image_host.clone(), host(true)];
        for _ in 0..50 {
            assert_eq!(pick_image_host(&hosts).unwrap().uuid, image_host.uuid);
        }
    }

    #[test]
    fn test_is_listing_not_empty() {
        let entry = DirectoryEntry {
            name: "backup".to_string(),
            is_dir: true,
            size: 0,
        };
        assert!(is_listing_not_empty(Ok(vec![entry])).unwrap());
        assert!(!is_listing_not_empty(Ok(vec![])).unwrap());
        // a host, which never stored a backup, has no backup-directory
        let not_found = AinariError::NotFound("missing".to_string());
        assert!(!is_listing_not_empty(Err(not_found)).unwrap());
        assert!(is_listing_not_empty(Err(AinariError::internal("broken"))).is_err());
    }

    #[test]
    fn test_pick_backup_host() {
        let backup_host = host(true);
        let hosts = vec![host(false), backup_host.clone(), host(false)];
        for _ in 0..50 {
            assert_eq!(pick_backup_host(&hosts).unwrap().uuid, backup_host.uuid);
        }
        assert!(pick_backup_host(&[]).is_none());
        assert!(pick_backup_host(&[host(false)]).is_none());
    }

    #[test]
    fn test_join_path() {
        assert_eq!(join_path("", "manifest.json"), "manifest.json");
        assert_eq!(
            join_path("metadata", "miko.json.enc"),
            "metadata/miko.json.enc"
        );
        assert_eq!(join_path("backups/x", "disks"), "backups/x/disks");
    }

    #[test]
    fn test_pick_image_host_without_image_hosts() {
        assert!(pick_image_host(&[]).is_none());
        assert!(pick_image_host(&[host(true), host(true)]).is_none());
    }
}
