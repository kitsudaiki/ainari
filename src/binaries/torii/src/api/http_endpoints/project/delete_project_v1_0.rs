// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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

use crate::database::project_table;

use ainari_api::errors::ErrorResponse;
use ainari_api::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "project",
    summary = "Delete project",
    description = r###"Delete a project from the database. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_project(
    project_id: Path<String>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    if !context.is_admin {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }

    // get new created project from database to get addtional information
    match project_table::delete_project(&project_id, &context) {
        Ok(_) => {
            return Ok(NoContent);
        }
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Project with ID '{project_id}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };
}
