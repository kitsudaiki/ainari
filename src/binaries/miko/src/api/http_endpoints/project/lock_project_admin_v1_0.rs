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
use uuid::Uuid;
use validator::Validate;

use crate::database::project_table::{self, LockResult};

use ainari_api::common_functions::*;
use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Lock project (admin)",
    description = r###"Lock a project manually.

While a project is locked, its users can only read its resources, all other requests of users to
the project are rejected by all components. Also no backup or restore can be started for the
project. `reason` describes, why the admin has locked the project. The lock gets the nil-UUID as
`locked_by_job`, because it belongs to no job, and it is only removed again by an admin with
`DELETE /v1alpha/project/{project_id}/lock/admin`. A project, which is already locked by a backup or
restore, is rejected with `409`. Locking a project again, which is already locked by an admin, is
accepted. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn lock_project_admin(
    project_id: Path<String>,
    body: Json<ProjectLockAdminReq>,
    context: UserContext,
) -> Result<Json<ProjectLockResp>, ErrorResponse> {
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // the lock belongs to no job, so it gets the nil-UUID, which the jobs are not allowed to use
    let lock_uuid = Uuid::nil();
    let result = project_table::lock_project(&project_id, &lock_uuid, &body.reason, &context)
        .map_err(|e| map_db_id_get_delete_error("project", &project_id, e, Some(&context)))?;
    if result == LockResult::LockedByOtherJob {
        return Err(ErrorResponse::Conflict(format!(
            "Project '{project_id}' is already locked by a backup or restore."
        )));
    }

    Ok(Json(super::lock_resp(&project_id, &context)?))
}
