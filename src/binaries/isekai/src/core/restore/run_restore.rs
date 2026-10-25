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

use std::collections::BTreeMap;
use std::collections::HashMap;

use ainari_api_structs::backup_structs::BackupKind;
use ainari_api_structs::network_structs::HanamiProjectExport;
use ainari_api_structs::project_structs::OmamoriProjectExport;
use ainari_clients::backup as backup_clients;
use ainari_common::error::AinariError;

use super::{BackupMetadata, RestoreJob, Restorer};
use crate::core::bundle::*;
use crate::core::keys::BackupKeys;
use crate::core::processing::jobs::Job;
use crate::core::services::Services;

/// Restores a backup into the project of the job.
///
/// # Arguments
///
/// * `job` - The job, which has locked the project
/// * `restore` - The restore
///
/// # Returns
///
/// Ok, if the backup was restored and the project unlocked again, or the error, which made it
/// fail.
pub async fn run_restore(job: &Job, restore: &RestoreJob) -> Result<(), AinariError> {
    let services = Services::connect(&job.context).await?;

    job.step("verify backup");
    let metadata = match verify_backup(job, restore, &services).await {
        Ok(metadata) => metadata,
        Err(e) => {
            // nothing was changed yet, so the project doesn't have to stay locked
            if let Err(unlock_error) = services.unlock(&job.project_id, &job.uuid).await {
                log::error!(
                    "Failed to unlock project '{}' after the failed restore {}: {unlock_error}",
                    job.project_id,
                    job.uuid
                );
            }
            return Err(e);
        }
    };

    let mut restorer = Restorer {
        job,
        restore,
        services: &services,
        metadata: &metadata,
        id_map: BTreeMap::new(),
    };
    for virtual_machine in &metadata.sakura.virtual_machines {
        restorer.restore_virtual_machine(virtual_machine).await?;
    }

    job.step("unlock project");
    services.unlock(&job.project_id, &job.uuid).await
}

/// Checks all files of the backup against its manifest and decrypts its metadata.
///
/// # Returns
///
/// The metadata, or `InvalidInput`, if a file is missing or was changed.
async fn verify_backup(
    job: &Job,
    restore: &RestoreJob,
    services: &Services,
) -> Result<BackupMetadata, AinariError> {
    if restore.manifest.kind != BackupKind::VirtualMachine {
        return Err(AinariError::InvalidInput(
            "Only backups of single virtual_machines can be restored at the moment.".to_string(),
        ));
    }

    let mut metadata_files = HashMap::new();
    for file in &restore.manifest.files {
        if file.path.starts_with("metadata/") {
            let content = backup_clients::read_backup_file(
                &services.endpoints.ryokan,
                &services.token,
                services.key(),
                &job.backup_uuid,
                &file.path,
                services.insecure(),
            )
            .await?;
            restore.manifest.check_content(&file.path, &content)?;
            metadata_files.insert(file.path.clone(), content);
        } else {
            let info = backup_clients::get_backup_file_info(
                &services.endpoints.ryokan,
                &services.token,
                services.key(),
                &job.backup_uuid,
                &file.path,
                services.insecure(),
            )
            .await
            .map_err(|e| match e {
                AinariError::NotFound(_) => AinariError::InvalidInput(format!(
                    "File '{}' of the backup is missing.",
                    file.path
                )),
                e => e,
            })?;
            restore
                .manifest
                .check_file(&file.path, info.size, &info.sha256)?;
        }
    }

    let omamori: OmamoriProjectExport =
        decrypt_file(&metadata_files, OMAMORI_METADATA, &restore.keys)?;
    let hanami: HanamiProjectExport =
        decrypt_file(&metadata_files, HANAMI_METADATA, &restore.keys)?;
    let sakura: SakuraMetadata = decrypt_file(&metadata_files, SAKURA_METADATA, &restore.keys)?;

    // the disks must be part of the checked files, else they could be swapped
    for virtual_machine in &sakura.virtual_machines {
        if restore.manifest.file(&virtual_machine.disk.path).is_none() {
            return Err(AinariError::InvalidInput(format!(
                "Disk '{}' is not part of the backup.",
                virtual_machine.disk.path
            )));
        }
    }

    Ok(BackupMetadata {
        omamori,
        hanami,
        sakura,
    })
}

/// Decrypts a metadata-file, which was read from the backup.
///
/// # Returns
///
/// The content of the file, or `InvalidInput`, if it is missing or can't be decrypted.
fn decrypt_file<T: serde::de::DeserializeOwned>(
    metadata_files: &HashMap<String, Vec<u8>>,
    path: &str,
    keys: &BackupKeys,
) -> Result<T, AinariError> {
    let content = metadata_files.get(path).ok_or_else(|| {
        AinariError::InvalidInput(format!("File '{path}' is missing in the backup."))
    })?;
    decrypt_metadata(path, content, keys)
}
