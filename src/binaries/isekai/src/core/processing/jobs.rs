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

use uuid::Uuid;

use ainari_api::omikuji_logging::report_internal_error;
use ainari_api_structs::backup_structs::{JobState, JobType};
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use crate::core::backup::{BackupJob, run_backup};
use crate::core::restore::{RestoreJob, run_restore};
use crate::database::job_table::{self, JobEntry};

/// The concrete work of a job together with everything it needs for it.
///
/// It contains the passphrase or the keys of the backup, which only exist in the memory of the
/// job, so a job can't be continued after a restart of isekai.
#[derive(Debug)]
pub enum JobVariant {
    Backup(BackupJob),
    Restore(Box<RestoreJob>),
}

#[derive(Debug)]
pub struct Job {
    /// Unique identifier of the job, which also locks the project
    pub uuid: Uuid,
    /// Backup, which is created or restored
    pub backup_uuid: Uuid,
    /// Project, which is saved or restored. It also decides, which worker-thread processes the
    /// job, so all jobs of the same project are serialized.
    pub project_id: String,
    /// Context of the user, who has started the job, whose token is used for all calls
    pub context: UserContext,
    pub variant: JobVariant,
}

impl Job {
    /// Type of the job.
    pub fn job_type(&self) -> JobType {
        match self.variant {
            JobVariant::Backup(_) => JobType::Backup,
            JobVariant::Restore(_) => JobType::Restore,
        }
    }

    /// Sets the step of the job, which is processed right now, so the user can follow it.
    ///
    /// # Arguments
    ///
    /// * `step` - Description of the step
    pub fn step(&self, step: &str) {
        log::info!("Job {} ({}): {step}", self.uuid, self.job_type());
        if job_table::set_job_step(&self.uuid, step).is_err() {
            log::error!("Failed to update the step of job {}", self.uuid);
        }
    }
}

/// Processes a job and stores its result in the database.
///
/// # Arguments
///
/// * `job` - The job
pub async fn process_job(job: Job) {
    // the job could have been marked as failed in the meantime, like by a restart
    let still_queued = job_table::get_job(&job.uuid, &admin_context())
        .ok()
        .and_then(|entry: JobEntry| entry.state().ok())
        == Some(JobState::Queued);
    if !still_queued {
        log::warn!("Skip job {}, because it is not queued anymore", job.uuid);
        return;
    }
    let _ = job_table::update_job_state(&job.uuid, JobState::Active);

    let result = match &job.variant {
        JobVariant::Backup(backup) => run_backup(&job, backup).await,
        JobVariant::Restore(restore) => run_restore(&job, restore).await,
    };

    match result {
        Ok(()) => {
            job.step("finished");
            let _ = job_table::update_job_state(&job.uuid, JobState::Finished);
        }
        Err(e) => {
            let message = e.to_string();
            match e {
                // the job is processed in the background, so its error is never converted into a
                // response and has to be reported here
                AinariError::InternalError(details) => {
                    let details = if details.user_id.is_empty() {
                        details.with_ids(&job.context.user_id, &job.context.project_id)
                    } else {
                        details
                    };
                    report_internal_error(details.with_message_prefix(&format!(
                        "Job {} ({}) failed: ",
                        job.uuid,
                        job.job_type()
                    )));
                }
                _ => log::error!("Job {} ({}) failed: {message}", job.uuid, job.job_type()),
            }
            let _ = job_table::add_message_to_job(&job.uuid, &message);
            let _ = job_table::update_job_state(&job.uuid, JobState::Error);
        }
    }
}

/// Marks all jobs as failed, which were queued or active, when isekai was stopped.
///
/// The passphrases and the tokens of the jobs only existed in the memory, so the jobs can't be
/// continued and nothing can be cleaned up in the other components right now. The incomplete
/// backup of a failed backup-job is marked as failed: its files are removed and its project is
/// unlocked, when the user deletes it. The project of a failed restore-job stays locked, like
/// after every failed restore, until the restore is repeated or an admin unlocks it.
pub fn fail_unfinished_jobs() {
    let failed = match job_table::fail_unfinished_jobs("Failed because of a restart of isekai.") {
        Ok(failed) => failed,
        Err(e) => {
            log::error!("Failed to mark the unfinished jobs as failed: {e}");
            return;
        }
    };
    if let Err(e) = crate::database::backup_table::fail_unfinished_backups() {
        log::error!("Failed to mark the unfinished backups as failed: {e}");
    }

    for job in failed {
        match job.job_type() {
            Ok(JobType::Backup) => log::warn!(
                "Backup-job {} failed because of the restart. Backup {} of project '{}' has to be \
                 deleted, which also unlocks the project again.",
                job.uuid,
                job.backup_uuid,
                job.project_id
            ),
            _ => log::warn!(
                "Restore-job {} failed because of the restart. Project '{}' stays locked, until \
                 the restore is repeated or an admin unlocks it.",
                job.uuid,
                job.project_id
            ),
        }
    }
}

/// Context, which can read all jobs.
fn admin_context() -> UserContext {
    UserContext {
        token: String::new(),
        user_id: String::new(),
        project_id: String::new(),
        is_admin: true.to_string(),
        project_role: String::new(),
    }
}
