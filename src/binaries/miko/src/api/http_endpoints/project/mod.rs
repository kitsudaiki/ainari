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

pub mod add_user_to_project_admin_v1_0;
pub mod create_project_admin_v1_0;
pub mod delete_project_admin_v1_0;
pub mod export_project_internal_v1_0;
pub mod get_project_admin_v1_0;
pub mod import_project_internal_v1_0;
pub mod list_project_admin_v1_0;
pub mod list_users_in_project_admin_v1_0;
pub mod list_users_in_project_v1_0;
pub mod lock_project_admin_v1_0;
pub mod lock_project_internal_v1_0;
pub mod remove_user_from_project_admin_v1_0;
pub mod unlock_project_admin_v1_0;
pub mod unlock_project_internal_v1_0;

use crate::database::project_table::{self, ProjectEntry};

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::{ProjectLock, ProjectLockResp};
use ainari_api_structs::user_context::UserContext;

/// Returns the lock of a project from the database as value of the api.
///
/// # Arguments
///
/// * `project` - The project from the database
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(Option<ProjectLock>)` - The lock, or None, if the project is not locked
/// * `Err(ErrorResponse::InternalError)` - The database contains an invalid lock
fn lock_of(
    project: &ProjectEntry,
    context: &UserContext,
) -> Result<Option<ProjectLock>, ErrorResponse> {
    project_table::lock_of(project).map_err(|e| {
        ainari_api::report_internal_error!(Some(context), "{e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })
}

/// Returns the current lock of a project as response of the api.
///
/// # Arguments
///
/// * `project_id` - The ID of the project
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(ProjectLockResp)` - The lock of the project
/// * `Err(ErrorResponse)` - The project doesn't exist or the database failed
fn lock_resp(project_id: &String, context: &UserContext) -> Result<ProjectLockResp, ErrorResponse> {
    let project = project_table::get_auth_project(project_id)
        .map_err(|e| map_db_id_get_delete_error("project", project_id, e, Some(context)))?;
    Ok(ProjectLockResp {
        project_id: project_id.clone(),
        lock: lock_of(&project, context)?,
    })
}

/// Checks, that a job doesn't use the nil-UUID, which marks the lock of an admin.
///
/// # Arguments
///
/// * `job_uuid` - The UUID of the job
///
/// # Returns
///
/// * `Ok(())` - The UUID can be used by a job
/// * `Err(ErrorResponse::BadRequest)` - The UUID is the nil-UUID
fn check_job_uuid(job_uuid: &uuid::Uuid) -> Result<(), ErrorResponse> {
    if job_uuid.is_nil() {
        return Err(ErrorResponse::BadRequest(
            "The nil-UUID is reserved for the lock of an admin and can't be used by a job."
                .to_string(),
        ));
    }
    Ok(())
}
