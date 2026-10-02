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
use apistos::actix::NoContent;
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
    summary = "Unassign project from user",
    description = r###"Remove the assignment of a project from a user. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn unassign_project_admin(
    user_id: Path<String>,
    body: Json<UserUnassignProjectReq>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let project_id = &body.project_id;

    // check if user exist
    user_table::get_user(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", &user_id, e))?;

    // mark mapping as deleted in database
    user_project_mapping_table::delete_mapping(project_id, &user_id, &context).map_err(
        |e| match e {
            DbError::NotFound => ErrorResponse::NotFound(format!(
                "User '{user_id}' is not assigned to project '{project_id}'."
            )),
            DbError::InternalError => {
                log::error!(
                    "Failed to unassign project '{project_id}' from user '{user_id}' in database."
                );
                ErrorResponse::InternalError("Internal Error".to_string())
            }
        },
    )?;

    Ok(NoContent)
}
