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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;

use crate::core::project_handling;
use crate::database::project_table;
use crate::database::user_project_mapping_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::ProjectRole;

#[api_operation(
    tag = "project",
    summary = "Delete project",
    description = r###"Delete a project together with its quota and the assignments of its users. Only projects without any resources and without users with the role admin or member can be deleted, so nobody can create a new resource during the delete. Observers don't block the delete. The default-project of a user can not be deleted here, it is only deleted together with its user. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn delete_project_admin(
    project_id: Path<String>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;

    // the default-project of a user is only deleted together with the user, so every user
    // always has its default-project
    if project_table::is_default_project(&project_id) {
        return Err(ErrorResponse::Conflict(format!(
            "Project '{project_id}' is the default-project of a user and can only be deleted \
             together with the user."
        )));
    }

    // check if project exist
    project_table::get_project(&project_id, &context)
        .map_err(|e| map_db_id_get_delete_error("project", &project_id, e))?;

    // Users, who could create resources, have to be removed or made observers first. Otherwise
    // a resource could be created between the check of the resources below and the delete.
    let active_users: Vec<String> =
        user_project_mapping_table::list_mappings_of_project(&project_id)
            .map_err(|e| map_db_list_error("user-project-mappings", e))?
            .into_iter()
            .filter(|mapping| matches!(mapping.role, ProjectRole::Admin | ProjectRole::Member))
            .map(|mapping| format!("{} ({})", mapping.user_id, mapping.role))
            .collect();
    if !active_users.is_empty() {
        return Err(ErrorResponse::Conflict(format!(
            "Project '{project_id}' still has users with the role admin or member ({}). They \
             have to be removed from the project or made observers first.",
            active_users.join(", ")
        )));
    }

    // only empty projects are allowed to be deleted
    let resources = project_handling::list_resources_of_project(&project_id, &context).await?;
    if !resources.is_empty() {
        return Err(ErrorResponse::Conflict(format!(
            "Project '{project_id}' still contains resources ({}). Only projects without \
             resources can be deleted.",
            resources.join(", ")
        )));
    }

    project_handling::delete_project_completely(&project_id, &context)?;

    Ok(NoContent)
}
