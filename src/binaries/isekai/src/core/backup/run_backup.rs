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

use chrono::Utc;

use ainari_api_structs::backup_structs::BackupStatus;
use ainari_clients::backup as backup_clients;
use ainari_clients::network::export_project_networks;
use ainari_clients::project::export_project;
use ainari_clients::secret::export_project_secrets;
use ainari_common::error::AinariError;

use super::backup_files::{delete_backup_content, file_info, write_metadata};
use super::collect_metadata::{
    check_references, collect_images, filter_resources_of_virtual_machine,
    filter_secrets_of_virtual_machines,
};
use super::save_virtual_machines::save_virtual_machines;
use super::{BackupJob, BackupScope};
use crate::config;
use crate::core::bundle::*;
use crate::core::manifest::{FORMAT_VERSION, MANIFEST_FILE, Manifest, ManifestKdf};
use crate::core::processing::jobs::Job;
use crate::core::services::Services;
use crate::database::backup_table::{self, ainari_version};
use crate::database::job_table;

/// Creates a backup.
///
/// On success the backup is sealed in ryokan and the project is unlocked again. If the backup
/// fails, the incomplete backup is deleted together with the secrets of its disks and the project
/// is unlocked as well, because the project itself wasn't changed by the backup.
///
/// # Arguments
///
/// * `job` - The job, which has locked the project
/// * `backup` - The backup
///
/// # Returns
///
/// Ok, if the backup is ready, or the error, which made it fail.
pub async fn run_backup(job: &Job, backup: &BackupJob) -> Result<(), AinariError> {
    let services = match Services::connect(&job.context).await {
        Ok(services) => services,
        Err(e) => {
            // without the other components nothing can be cleaned up, so the incomplete backup
            // has to be deleted by the user and the project unlocked by an admin
            let _ = backup_table::update_backup_state(&job.backup_uuid, BackupStatus::Error);
            return Err(e);
        }
    };

    if let Err(e) = create_backup(job, backup, &services).await {
        cleanup_failed_backup(job, &services).await;
        return Err(e);
    }

    job.step("unlock project");
    services.unlock(&job.project_id, &job.uuid).await
}

/// Processes all steps of a backup until it is sealed.
async fn create_backup(
    job: &Job,
    backup: &BackupJob,
    services: &Services,
) -> Result<(), AinariError> {
    job.step("derive keys");
    let kdf = ManifestKdf::new(config::CONFIG.kdf.params());
    let keys = kdf.derive_keys(&backup.passphrase)?;

    job.step("collect resources");
    let mut hanami = export_project_networks(
        &services.endpoints.hanami,
        &services.token,
        services.key(),
        &job.project_id,
        services.insecure(),
    )
    .await?;
    if let BackupScope::VirtualMachine(virtual_machine_uuid) = backup.scope {
        hanami = filter_resources_of_virtual_machine(hanami, &virtual_machine_uuid)?;
    }

    job.step("save root-disks");
    let sakura = save_virtual_machines(job, backup, services, &hanami.virtual_machines).await?;

    let ryokan = match backup.scope {
        BackupScope::Project => Some(collect_images(job, backup, services).await?),
        BackupScope::VirtualMachine(_) => None,
    };

    job.step("collect metadata");
    let miko = match backup.scope {
        BackupScope::Project => Some(
            export_project(
                &services.miko,
                &services.token,
                services.key(),
                &job.project_id,
                services.insecure(),
            )
            .await?,
        ),
        BackupScope::VirtualMachine(_) => None,
    };
    let mut omamori = export_project_secrets(
        &services.endpoints.omamori,
        &services.token,
        services.key(),
        &job.project_id,
        &keys.wrap_key_base64(),
        Some(&job.backup_uuid),
        services.insecure(),
    )
    .await?;
    if let BackupScope::VirtualMachine(_) = backup.scope {
        omamori = filter_secrets_of_virtual_machines(omamori, &sakura);
    }
    for missing in check_references(&omamori, &sakura, ryokan.as_ref())? {
        let _ = job_table::add_message_to_job(&job.uuid, &missing);
    }

    job.step("write metadata");
    let mut files = Vec::new();
    if let Some(miko) = &miko {
        files.push(write_metadata(job, services, MIKO_METADATA, miko, &keys).await?);
    }
    files.push(write_metadata(job, services, OMAMORI_METADATA, &omamori, &keys).await?);
    if let Some(ryokan) = &ryokan {
        files.push(write_metadata(job, services, RYOKAN_METADATA, ryokan, &keys).await?);
    }
    files.push(write_metadata(job, services, HANAMI_METADATA, &hanami, &keys).await?);
    files.push(write_metadata(job, services, SAKURA_METADATA, &sakura, &keys).await?);

    job.step("write manifest");
    for virtual_machine in &sakura.virtual_machines {
        files.push(file_info(job, services, &virtual_machine.disk.path).await?);
    }
    if let Some(ryokan) = ryokan.as_ref().filter(|ryokan| ryokan.files_included) {
        for image in &ryokan.images {
            files.push(file_info(job, services, &image_path(&image.uuid)).await?);
        }
    }
    let manifest = Manifest {
        format_version: FORMAT_VERSION,
        ainari_version: ainari_version(),
        kind: backup.scope.kind(),
        backup_uuid: job.backup_uuid,
        project_id: job.project_id.clone(),
        virtual_machine_uuid: backup.scope.virtual_machine_uuid(),
        created_at: Utc::now(),
        kdf,
        files,
    };
    // the manifest is written last, so a backup without manifest is known to be incomplete
    backup_clients::write_backup_file(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &job.backup_uuid,
        MANIFEST_FILE,
        &manifest.seal(&keys)?,
        services.insecure(),
    )
    .await?;

    job.step("finalize backup");
    backup_clients::finalize_backup(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &job.backup_uuid,
        services.insecure(),
    )
    .await?;
    let finished = backup_table::change_backup_state(
        &job.backup_uuid,
        BackupStatus::Creating,
        BackupStatus::Ready,
    )
    .map_err(|_| AinariError::internal("Failed to update the state of the backup"))?;
    if !finished {
        return Err(AinariError::Conflict(format!(
            "Backup '{}' was changed, while it was created.",
            job.backup_uuid
        )));
    }

    Ok(())
}

/// Removes everything of a failed backup and unlocks the project again.
///
/// Every part is cleaned up independently, so one failure doesn't leave the rest behind. The
/// errors are only logged, because the error of the backup itself is more important.
async fn cleanup_failed_backup(job: &Job, services: &Services) {
    job.step("clean up failed backup");
    if backup_table::update_backup_state(&job.backup_uuid, BackupStatus::Error).is_err() {
        log::error!("Failed to mark backup {} as failed", job.backup_uuid);
    }

    match backup_table::get_backup(&job.backup_uuid, &job.context) {
        // the errors are already logged
        Ok(entry) => {
            let _ = delete_backup_content(services, &entry).await;
        }
        Err(_) => log::error!("Failed to read backup {} for its cleanup", job.backup_uuid),
    }

    if let Err(e) = services.unlock(&job.project_id, &job.uuid).await {
        log::error!(
            "Failed to unlock project '{}' after the failed backup {}: {e}",
            job.project_id,
            job.backup_uuid
        );
    }
}
