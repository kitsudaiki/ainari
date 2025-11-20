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

use crate::database::project_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Get project",
    description = r###"Get information of a project from the database. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_project_admin(
    project_id: Path<String>,
    context: UserContext,
) -> Result<Json<ProjectResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;

    // get project from database
    let project = project_table::get_project(&project_id, &context)
        .map_err(|e| map_db_id_get_delete_error("project", &project_id, e))?;

    let resp = ProjectResp {
        id: project.id,
        name: project.name,
        created_by: project.created_by,
        created_at: project.created_at,
        updated_by: project.updated_by,
        updated_at: project.updated_at,
    };

    Ok(Json(resp))
}
