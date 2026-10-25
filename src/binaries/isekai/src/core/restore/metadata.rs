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

use std::collections::HashMap;
use uuid::Uuid;

use ainari_clients::backup as backup_clients;
use ainari_common::error::AinariError;

use super::BackupMetadata;
use crate::core::bundle::*;
use crate::core::keys::BackupKeys;
use crate::core::manifest::Manifest;
use crate::core::services::Services;

/// Directory of the metadata-files within a backup
const METADATA_DIRECTORY: &str = "metadata/";

/// Reads the metadata-files of a backup, checks them against the manifest and decrypts them.
///
/// The metadata of miko and ryokan is only part of the backups of whole projects.
///
/// # Arguments
///
/// * `services` - Connection to the other components
/// * `backup_uuid` - The backup
/// * `manifest` - The authenticated manifest of the backup
/// * `keys` - Keys of the backup
///
/// # Returns
///
/// The metadata, or `InvalidInput`, if a file is missing, was changed or can't be decrypted.
pub async fn load_metadata(
    services: &Services,
    backup_uuid: &Uuid,
    manifest: &Manifest,
    keys: &BackupKeys,
) -> Result<BackupMetadata, AinariError> {
    let mut files = HashMap::new();
    for file in manifest
        .files
        .iter()
        .filter(|file| file.path.starts_with(METADATA_DIRECTORY))
    {
        let content = backup_clients::read_backup_file(
            &services.endpoints.ryokan,
            &services.token,
            services.key(),
            backup_uuid,
            &file.path,
            services.insecure(),
        )
        .await?;
        manifest.check_content(&file.path, &content)?;
        files.insert(file.path.clone(), content);
    }

    let metadata = BackupMetadata {
        miko: decrypt_optional(&files, MIKO_METADATA, keys)?,
        omamori: decrypt_required(&files, OMAMORI_METADATA, keys)?,
        ryokan: decrypt_optional(&files, RYOKAN_METADATA, keys)?,
        hanami: decrypt_required(&files, HANAMI_METADATA, keys)?,
        sakura: decrypt_required(&files, SAKURA_METADATA, keys)?,
    };

    // the disks and images must be part of the checked files, else they could be swapped
    let mut referenced: Vec<String> = metadata
        .sakura
        .virtual_machines
        .iter()
        .map(|virtual_machine| virtual_machine.disk.path.clone())
        .collect();
    if let Some(ryokan) = metadata
        .ryokan
        .as_ref()
        .filter(|ryokan| ryokan.files_included)
    {
        referenced.extend(ryokan.images.iter().map(|image| image_path(&image.uuid)));
    }
    for path in referenced {
        if manifest.file(&path).is_none() {
            return Err(AinariError::InvalidInput(format!(
                "File '{path}' is not part of the backup."
            )));
        }
    }

    Ok(metadata)
}

/// Checks the size and checksum of the files of a backup, which are not metadata, like the disks
/// and images, against the manifest.
///
/// The onsen reads the whole content of every file, so it takes its time for large disks.
///
/// # Arguments
///
/// * `services` - Connection to the other components
/// * `backup_uuid` - The backup
/// * `manifest` - The authenticated manifest of the backup
///
/// # Returns
///
/// Ok, if all files are unchanged, or `InvalidInput`, if a file is missing or was changed.
pub async fn verify_files(
    services: &Services,
    backup_uuid: &Uuid,
    manifest: &Manifest,
) -> Result<(), AinariError> {
    for file in manifest
        .files
        .iter()
        .filter(|file| !file.path.starts_with(METADATA_DIRECTORY))
    {
        let info = backup_clients::get_backup_file_info(
            &services.endpoints.ryokan,
            &services.token,
            services.key(),
            backup_uuid,
            &file.path,
            services.insecure(),
        )
        .await
        .map_err(|e| match e {
            AinariError::NotFound(_) => {
                AinariError::InvalidInput(format!("File '{}' of the backup is missing.", file.path))
            }
            e => e,
        })?;
        manifest.check_file(&file.path, info.size, &info.sha256)?;
    }
    Ok(())
}

/// Decrypts a metadata-file, which every backup contains.
fn decrypt_required<T: serde::de::DeserializeOwned>(
    files: &HashMap<String, Vec<u8>>,
    path: &str,
    keys: &BackupKeys,
) -> Result<T, AinariError> {
    decrypt_optional(files, path, keys)?.ok_or_else(|| {
        AinariError::InvalidInput(format!("File '{path}' is missing in the backup."))
    })
}

/// Decrypts a metadata-file, which is only part of some backups.
fn decrypt_optional<T: serde::de::DeserializeOwned>(
    files: &HashMap<String, Vec<u8>>,
    path: &str,
    keys: &BackupKeys,
) -> Result<Option<T>, AinariError> {
    files
        .get(path)
        .map(|content| decrypt_metadata(path, content, keys))
        .transpose()
}
