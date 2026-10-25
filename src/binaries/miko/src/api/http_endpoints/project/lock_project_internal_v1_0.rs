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

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;
use validator::Validate;

use crate::database::project_table::{self, LockResult};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Lock project",
    description = r###"Lock a project for a job, like a backup or a restore. `reason` describes, why
the project is locked.

While a project is locked, its users can only read its resources, all other requests of users to
the project are rejected by all components. Only the job, which has locked the project, can remove
the lock again. Locking a project, which is already locked by another job, is rejected with `409`,
so two backups or restores of the same project can't run at the same time. Locking it again with the
same job is accepted. The nil-UUID is reserved for the lock of an admin and is rejected with
`400`. Users, which are not admin, can only lock their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn lock_project_internal(
    project_id: Path<String>,
    body: Json<ProjectLockReq>,
    context: UserContext,
) -> Result<Json<ProjectLockResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    super::check_project_access(&project_id, &context)?;
    super::check_job_uuid(&body.job_uuid)?;

    let result =
        project_table::lock_project(&project_id, &body.job_uuid, &body.reason, &context)
            .map_err(|e| map_db_id_get_delete_error("project", &project_id, e, Some(&context)))?;
    if result == LockResult::LockedByOtherJob {
        return Err(ErrorResponse::Conflict(format!(
            "Project '{project_id}' is already locked by another job."
        )));
    }

    Ok(Json(super::lock_resp(&project_id, &context)?))
}
