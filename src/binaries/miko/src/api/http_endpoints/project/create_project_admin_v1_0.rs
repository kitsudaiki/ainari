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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::database::project_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Create new project",
    description = r###"Create new project. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn create_project_admin(
    body: Json<ProjectCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ProjectResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let project_id = &body.id;

    // check if project-id already exist
    check_if_id_exist_in_db(
        "project",
        project_id,
        project_table::get_project(project_id, &context),
    )?;

    // add new project to datbase
    project_table::add_new_project(project_id, &body.name, &context).map_err(|e| {
        log::error!("Failed to add project with ID '{project_id}' to database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // get new created project from database to get addtional information
    let project = project_table::get_project(project_id, &context)
        .map_err(|e| map_db_id_get_delete_error("project", project_id, e))?;

    let resp = ProjectResp {
        id: project.id,
        name: project.name,
        created_by: project.created_by,
        created_at: project.created_at,
        updated_by: project.updated_by,
        updated_at: project.updated_at,
    };

    Ok(CreatedJson(resp))
}
