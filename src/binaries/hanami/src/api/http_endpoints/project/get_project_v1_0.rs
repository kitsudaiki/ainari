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
use actix_web::web::Path;
use apistos::api_operation;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::project_table;

use hanami_common::enums;
use hanami_structs::project_structs::ProjectResp;

#[api_operation(
    tag = "project",
    summary = "Get project",
    description = r###"Get information of a project from the database. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_project(project_id: Path<String>, context: UserContext) -> Result<Json<ProjectResp>, ErrorResponse> {
    if context.is_admin == false {
        return Err(ErrorResponse::Unauthorized("Only Admins are allowed to use this endpoint".to_string()));
    }

    // get new created project from database to get addtional information
    match project_table::get_project(&project_id, &context) {
        Ok(project) => {
            let resp = ProjectResp {
                id: project.id.clone(),
                name: project.name.clone(),
                created_by: project.created_by.clone(),
                created_at: project.created_at.clone(),
                updated_by: project.updated_by.clone(),
                updated_at: project.updated_at.clone(),
            };
        
            return Ok(Json(resp));
        },
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Project with ID '{project_id}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };
}