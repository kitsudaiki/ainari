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

pub mod create_project_backup_v1_0;
pub mod create_virtual_machine_backup_v1_0;
pub mod delete_backup_v1_0;
pub mod get_backup_v1_0;
pub mod list_backup_v1_0;

use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::backup as backup_clients;
use ainari_clients::virtual_machine::get_virtual_machine_host;
use ainari_common::error::AinariError;

use crate::core::backup::{BackupJob, BackupScope};
use crate::core::processing::jobs::{Job, JobVariant};
use crate::core::processing::worker_handler::add_job_to_queue;
use crate::core::services::Services;
use crate::database::backup_table::{self, BackupEntry};
use crate::database::id_map_table;
use crate::database::job_table::{self, JobEntry};

/// Starts a new backup of the project of the user.
///
/// The backup-resource is created in ryokan and the project is locked, before the job is queued,
/// so a project, which is already locked by another backup or restore, is rejected right away.
/// Everything, which was done before a failure, is reverted again.
///
/// # Arguments
///
/// * `name` - Name of the backup
/// * `backup` - Content and options of the backup
/// * `context` - User-context of the request
///
/// # Returns
///
/// The new backup together with the job, which creates it.
async fn start_backup(
    name: &str,
    backup: BackupJob,
    context: &UserContext,
) -> Result<BackupJobResp, ErrorResponse> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(permission_denied_response());
    }

    let services = Services::connect(context)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;

    // checks, that the virtual_machine exists and belongs to the project of the user
    if let BackupScope::VirtualMachine(virtual_machine_uuid) = backup.scope {
        get_virtual_machine_host(
            &services.endpoints.hanami,
            &services.token,
            services.key(),
            &virtual_machine_uuid,
            services.insecure(),
        )
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;
    }

    let backup_uuid = Uuid::new_v4();
    let job_uuid = Uuid::new_v4();
    let project_id = context.project_id.clone();

    // ryokan selects the backup-target, which stores the files of the backup
    backup_clients::create_backup(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &backup_uuid,
        &project_id,
        None,
        services.insecure(),
    )
    .await
    .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;

    let reason = format!("backup {backup_uuid}");
    if let Err(e) = services.lock(&project_id, &job_uuid, &reason).await {
        revert_start(&services, &backup_uuid, None, &project_id).await;
        return Err(map_ainari_error_to_api_response(e, Some(context)));
    }

    let stored = backup_table::add_new_backup(
        &backup_uuid,
        name,
        backup.scope.kind(),
        backup.scope.virtual_machine_uuid(),
        context,
    )
    .and_then(|_| {
        job_table::add_new_job(
            &job_uuid,
            JobType::Backup,
            &backup_uuid,
            &project_id,
            &backup.options(),
            context,
        )
    });
    if let Err(e) = stored {
        let _ = backup_table::update_backup_state(&backup_uuid, BackupStatus::Error);
        revert_start(&services, &backup_uuid, Some(&job_uuid), &project_id).await;
        return Err(map_db_write_error("add backup", e, Some(context)));
    }

    add_job_to_queue(Job {
        uuid: job_uuid,
        backup_uuid,
        project_id,
        context: context.clone(),
        variant: JobVariant::Backup(backup),
    });

    let backup_entry = backup_table::get_backup(&backup_uuid, context)
        .map_err(|e| map_db_uuid_get_after_add_error("backup", &backup_uuid, e, Some(context)))?;
    let job_entry = job_table::get_job(&job_uuid, context)
        .map_err(|e| map_db_uuid_get_after_add_error("job", &job_uuid, e, Some(context)))?;

    Ok(BackupJobResp {
        backup: to_backup_resp(backup_entry, None)?,
        job: to_job_resp(job_entry, context)?,
    })
}

/// Reverts the start of a backup, which failed before its job was queued.
///
/// # Arguments
///
/// * `services` - Connection to the other components
/// * `backup_uuid` - The backup-resource in ryokan
/// * `job_uuid` - The job, which has already locked the project
/// * `project_id` - The project
async fn revert_start(
    services: &Services,
    backup_uuid: &Uuid,
    job_uuid: Option<&Uuid>,
    project_id: &str,
) {
    if let Some(job_uuid) = job_uuid
        && let Err(e) = services.unlock(project_id, job_uuid).await
    {
        log::error!("Failed to unlock project '{project_id}' again: {e}");
    }
    match backup_clients::delete_backup(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        backup_uuid,
        services.insecure(),
    )
    .await
    {
        Ok(()) | Err(AinariError::NotFound(_)) => {}
        Err(e) => log::error!("Failed to delete backup {backup_uuid} in ryokan again: {e}"),
    }
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

/// Converts a job of the database into its response, together with the UUIDs, which were
/// changed by the job.
///
/// # Arguments
///
/// * `entry` - The job
/// * `context` - User-context of the request
///
/// # Returns
///
/// The response, or an internal error, if the database contains invalid values.
fn to_job_resp(entry: JobEntry, context: &UserContext) -> Result<JobResp, ErrorResponse> {
    let job_type = entry
        .job_type()
        .map_err(|e| map_internal_error("read type of job", e, Some(context)))?;
    let state = entry
        .state()
        .map_err(|e| map_internal_error("read state of job", e, Some(context)))?;
    let id_map = id_map_table::list_id_map_of_job(&entry.uuid)
        .map_err(|e| map_db_list_error("id-map", e, Some(context)))?
        .into_iter()
        .map(|mapping| IdMapEntryResp {
            resource_type: mapping.resource_type,
            old_uuid: mapping.old_uuid,
            new_uuid: mapping.new_uuid,
        })
        .collect();

    Ok(JobResp {
        uuid: entry.uuid,
        job_type,
        backup_uuid: entry.backup_uuid,
        project_id: entry.project_id,
        state,
        step: entry.step,
        messages: entry.messages,
        id_map,
        created_at: entry.created_at,
        created_by: entry.created_by,
        started_at: entry.started_at,
        finished_at: entry.finished_at,
    })
}
