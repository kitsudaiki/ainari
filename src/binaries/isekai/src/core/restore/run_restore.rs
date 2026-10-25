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

use ainari_api_structs::backup_structs::BackupKind;
use ainari_common::error::AinariError;

use super::metadata::verify_files;
use super::{RestoreJob, Restorer};
use crate::core::processing::jobs::Job;
use crate::core::services::Services;

/// Restores a backup into the project of the job.
///
/// The metadata of the backup was already checked and decrypted, before the job was queued. The
/// job checks the other files of the backup first, so a manipulated backup changes nothing. If the
/// restore fails afterwards, the project stays locked.
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
    if let Err(e) = verify_files(&services, &job.backup_uuid, &restore.manifest).await {
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

    let mut restorer = Restorer::new(job, restore, &services)?;
    match restore.manifest.kind {
        BackupKind::Project => restorer.restore_project().await?,
        BackupKind::VirtualMachine => {
            for virtual_machine in &restore.metadata.sakura.virtual_machines {
                restorer.restore_virtual_machine(virtual_machine).await?;
            }
        }
    }

    job.step("unlock project");
    services.unlock(&job.project_id, &job.uuid).await
}
