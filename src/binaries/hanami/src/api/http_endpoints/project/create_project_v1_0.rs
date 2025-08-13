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

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::project_table;

use ainari_common::enums;
use ainari_structs::project_structs::{ProjectCreateReq, ProjectResp};

#[api_operation(
    tag = "project",
    summary = "Create new project",
    description = r###"Create new project. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn create_project(
    body: Json<ProjectCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ProjectResp>, ErrorResponse> {
    if !context.is_admin {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }

    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    let id = &body.id;

    // check if project already exist within the database
    match project_table::get_project(id, &context) {
        Ok(_) => {
            let msg = format!("Project with ID '{id}' already exist.");
            return Err(ErrorResponse::Conflict(msg));
        }
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            // it is desired, that the project not already exist, so this error will be ignored
        }
    };

    // add new project to datbase
    match project_table::add_new_project(id, &body.name, &context) {
        Ok(_) => {}
        Err(e) => {
            log::error!("Failed to add project with ID '{id}' to database.: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created project from database to get addtional information
    match project_table::get_project(id, &context) {
        Ok(project) => {
            let resp = ProjectResp {
                id: project.id.clone(),
                name: project.name.clone(),
                created_by: project.created_by.clone(),
                created_at: project.created_at.clone(),
                updated_by: project.updated_by.clone(),
                updated_at: project.updated_at.clone(),
            };

            return Ok(CreatedJson(resp));
        }
        Err(_) => {
            let msg = format!(
                "Failed to get project with ID '{id}' from database, even the project should exist."
            );
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
}
