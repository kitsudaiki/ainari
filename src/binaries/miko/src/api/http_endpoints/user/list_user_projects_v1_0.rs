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
use apistos::api_operation;

use crate::database::user_project_mapping_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "user",
    summary = "List invited projects",
    description = r###"List all projects, to which the user of the token is assigned, together with the role of the user within each of these projects.

The default-project of the user is part of the list as well."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_user_projects(
    context: UserContext,
) -> Result<Json<ProjectInvitedListResp>, ErrorResponse> {
    // get all active assignments of the user of the token from database
    let mappings = user_project_mapping_table::list_mappings_of_user(&context.user_id)
        .map_err(|e| map_db_list_error("invited projects", e))?;

    let mut resp = ProjectInvitedListResp {
        projects: Vec::new(),
    };

    // convert database-output
    for mapping in mappings {
        let obj = ProjectInvitedResp {
            project_id: mapping.project_id,
            project_role: mapping.role,
        };

        resp.projects.push(obj);
    }

    Ok(Json(resp))
}
