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

use crate::database::user_project_mapping_table;
use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::user_structs::*;
use ainari_common::enums::DbError;

#[api_operation(
    tag = "user",
    summary = "Set project-role of user",
    description = r###"Set the role of a user within a project. The user must already be assigned to the project. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn set_project_role_admin(
    user_id: Path<String>,
    body: Json<UserSetProjectRoleReq>,
    context: UserContext,
) -> Result<Json<UserSetProjectRoleResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let project_id = &body.project_id;

    // check if user exist
    user_table::get_user(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", &user_id, e))?;

    // update role in database, which only works, if the user is already assigned to the project
    user_project_mapping_table::set_mapping_role(project_id, &user_id, body.project_role, &context)
        .map_err(|e| match e {
            DbError::NotFound => ErrorResponse::NotFound(format!(
                "User '{user_id}' is not assigned to project '{project_id}'."
            )),
            DbError::PermissionDenied => permission_denied_response(),
            DbError::InternalError => {
                log::error!(
                    "Failed to set role of user '{user_id}' in project '{project_id}' in database."
                );
                ErrorResponse::InternalError("Internal Error".to_string())
            }
        })?;

    // get updated mapping from database to get the stored values
    let mapping = user_project_mapping_table::get_mapping(project_id, &user_id)
        .map_err(|e| map_db_id_get_after_add_error("user-project-mapping", &user_id, e))?;

    let resp = UserSetProjectRoleResp {
        user_id: mapping.user_id,
        project_id: mapping.project_id,
        project_role: mapping.role,
    };

    Ok(Json(resp))
}
