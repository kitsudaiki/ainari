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

use serde::Serialize;

use ainari_api_structs::backup_structs::MAX_BACKUP_FILE_SIZE;
use ainari_clients::backup as backup_clients;
use ainari_clients::secret::delete_secret;
use ainari_common::enums;
use ainari_common::error::AinariError;

use crate::core::bundle::encrypt_metadata;
use crate::core::keys::BackupKeys;
use crate::core::manifest::{ManifestFile, sha256_hex};
use crate::core::processing::jobs::Job;
use crate::core::services::Services;
use crate::database::backup_table::{self, BackupEntry};

/// Encrypts a metadata-file and writes it into the backup.
pub(super) async fn write_metadata<T: Serialize>(
    job: &Job,
    services: &Services,
    path: &str,
    data: &T,
    keys: &BackupKeys,
) -> Result<ManifestFile, AinariError> {
    let content = encrypt_metadata(path, data, keys)?;
    if content.len() > MAX_BACKUP_FILE_SIZE {
        return Err(AinariError::internal(format!(
            "Metadata-file '{path}' has {} bytes, which is more than the maximum of \
             {MAX_BACKUP_FILE_SIZE} bytes",
            content.len()
        )));
    }

    backup_clients::write_backup_file(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &job.backup_uuid,
        path,
        &content,
        services.insecure(),
    )
    .await?;

    Ok(ManifestFile {
        path: path.to_string(),
        size: content.len() as u64,
        sha256: sha256_hex(&content),
    })
}

/// Reads size and checksum of a file, which was written into the backup by another component.
pub(super) async fn file_info(
    job: &Job,
    services: &Services,
    path: &str,
) -> Result<ManifestFile, AinariError> {
    let info = backup_clients::get_backup_file_info(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &job.backup_uuid,
        path,
        services.insecure(),
    )
    .await?;

    Ok(ManifestFile {
        path: info.path,
        size: info.size,
        sha256: info.sha256,
    })
}

/// Deletes the files of a backup in ryokan and the remaining secrets of its disks in omamori.
///
/// Parts, which are already deleted, are skipped, so it can be repeated.
///
/// # Arguments
///
/// * `services` - Connection to the other components
/// * `entry` - The backup
///
/// # Returns
///
/// Ok, if everything of the backup is deleted, or the first error.
pub async fn delete_backup_content(
    services: &Services,
    entry: &BackupEntry,
) -> Result<(), AinariError> {
    let result = match backup_clients::delete_backup(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &entry.uuid,
        services.insecure(),
    )
    .await
    {
        Ok(()) | Err(AinariError::NotFound(_)) => Ok(()),
        Err(e) => {
            log::error!("Failed to delete the files of backup {}: {e}", entry.uuid);
            Err(e)
        }
    };

    result.and(delete_backup_secrets(services, entry).await)
}

/// Deletes the secrets of the disks of a backup in omamori.
///
/// The payloads of the secrets are part of the metadata of the backup, so the secrets in omamori
/// are only needed by sakura, while it encrypts the disks. Every deleted secret is removed from the
/// backup, so a repetition only deletes the remaining ones.
///
/// # Arguments
///
/// * `services` - Connection to the other components
/// * `entry` - The backup
///
/// # Returns
///
/// Ok, if all secrets are deleted, or the first error.
pub async fn delete_backup_secrets(
    services: &Services,
    entry: &BackupEntry,
) -> Result<(), AinariError> {
    let mut result = Ok(());
    for secret_uuid in entry.secrets() {
        match delete_secret(
            &services.endpoints.omamori,
            &services.token,
            services.key(),
            &secret_uuid,
            services.insecure(),
        )
        .await
        {
            Ok(()) | Err(AinariError::NotFound(_)) => {
                // a backup, which was deleted in the meantime, has nothing to remove anymore
                if let Err(enums::DbError::InternalError) =
                    backup_table::remove_backup_secret(&entry.uuid, &secret_uuid)
                {
                    log::error!(
                        "Failed to remove the deleted secret {secret_uuid} from backup {}",
                        entry.uuid
                    );
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to delete secret {secret_uuid} of backup {}: {e}",
                    entry.uuid
                );
                result = result.and(Err(e));
            }
        }
    }

    result
}
