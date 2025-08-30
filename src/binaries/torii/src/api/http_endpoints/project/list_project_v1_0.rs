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
use apistos::api_operation;

use crate::database::project_table;

use super::project_structs::{ProjectBasicResp, ProjectListResp};

use ainari_api::errors::ErrorResponse;
use ainari_api::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "List project",
    description = r###"List basic information of all project from the database. This can only be done by an admin."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_project(context: UserContext) -> Result<Json<ProjectListResp>, ErrorResponse> {
    if !context.is_admin {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }

    let projects = match project_table::list_projects(&context) {
        Ok(result) => result,
        Err(e) => {
            let msg = format!("Failed to list projects with error: '{e}'");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let mut resp = ProjectListResp {
        projects: Vec::new(),
    };

    for project in projects {
        let obj = ProjectBasicResp {
            id: project.id.clone(),
            name: project.name.clone(),
        };

        resp.projects.push(obj); // fill the vector with objects
    }

    Ok(Json(resp))
}
