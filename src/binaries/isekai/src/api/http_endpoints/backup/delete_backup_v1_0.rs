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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use crate::core::backup::delete_backup_content;
use crate::core::services::Services;
use crate::database::{backup_table, job_table};

#[api_operation(
    tag = "backup",
    summary = "Delete backup",
    description = r###"Delete a backup together with its files and the secrets of its disks.

Only admins of the project can delete a backup. A backup, which is created or restored right now,
is rejected with `409`. If the deletion fails, the backup stays in the state `DELETING` and the
deletion can be repeated.

The project of a backup, whose creation was interrupted by a restart of isekai, is unlocked again
by the deletion."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn delete_backup(
    backup_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    super::super::check_project_admin(&context)?;
    let entry = backup_table::get_backup(&backup_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;

    let jobs = job_table::list_jobs_of_backup(&backup_uuid)
        .map_err(|e| map_db_list_error("jobs", e, Some(&context)))?;
    if jobs
        .iter()
        .any(|job| job.state().is_ok_and(|state| !state.is_done()))
    {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' is used by a running job.",
            *backup_uuid
        )));
    }

    backup_table::update_backup_state(&backup_uuid, BackupStatus::Deleting)
        .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;

    let services = Services::connect(&context)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    delete_backup_content(&services, &entry)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    // a backup-job, which was interrupted, couldn't unlock the project anymore. The lock can only
    // be removed by the job, which has set it, so this never removes the lock of another job.
    for job in jobs
        .iter()
        .filter(|job| job.job_type() == Ok(JobType::Backup) && job.state() == Ok(JobState::Error))
    {
        match services.unlock(&job.project_id, &job.uuid).await {
            Ok(()) | Err(AinariError::Conflict(_)) | Err(AinariError::NotFound(_)) => {}
            Err(e) => log::warn!(
                "Failed to remove the lock of job {} from project '{}': {e}",
                job.uuid,
                job.project_id
            ),
        }
    }

    backup_table::delete_backup(&backup_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;

    Ok(NoContent)
}
