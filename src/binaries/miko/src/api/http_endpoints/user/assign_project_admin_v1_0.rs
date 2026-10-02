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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use diesel::result::{DatabaseErrorKind, Error};
use validator::Validate;

use crate::database::project_table;
use crate::database::user_project_mapping_table;
use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::user_structs::*;

#[api_operation(
    tag = "user",
    summary = "Assign project to user",
    description = r###"Assign a project with a specific role to a user. A user can be assigned to the same project only once at the same time. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn assign_project_admin(
    user_id: Path<String>,
    body: Json<UserAssignProjectReq>,
    context: UserContext,
) -> Result<CreatedJson<UserAssignProjectResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let project_id = &body.project_id;

    // check if user and project exist
    user_table::get_user(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", &user_id, e))?;
    project_table::get_project(project_id, &context)
        .map_err(|e| map_db_id_get_delete_error("project", project_id, e))?;

    // add new mapping to database. The unique index of the table rejects a second active mapping
    // of the same pair, so this doesn't have to be checked before.
    user_project_mapping_table::add_new_mapping(project_id, &user_id, body.project_role, &context)
        .map_err(|e| match e {
            Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => ErrorResponse::Conflict(
                format!("User '{user_id}' is already assigned to project '{project_id}'."),
            ),
            e => map_db_write_error(
                &format!("assign project '{project_id}' to user '{user_id}' in database"),
                e,
            ),
        })?;

    // get new created mapping from database to get the stored values
    let mapping = user_project_mapping_table::get_mapping(project_id, &user_id)
        .map_err(|e| map_db_id_get_after_add_error("user-project-mapping", &user_id, e))?;

    let resp = UserAssignProjectResp {
        user_id: mapping.user_id,
        project_id: mapping.project_id,
        project_role: mapping.role,
    };

    Ok(CreatedJson(resp))
}
