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
    tag = "project",
    summary = "List users in project",
    description = r###"List all users, which are assigned to the project of the token, together with the role of each user within this project."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_users_in_project(
    context: UserContext,
) -> Result<Json<ProjectMemberListResp>, ErrorResponse> {
    // get all active assignments of the project of the token from database
    let mappings = user_project_mapping_table::list_mappings_of_project(&context.project_id)
        .map_err(|e| map_db_list_error("project members", e))?;

    let mut resp = ProjectMemberListResp {
        members: Vec::new(),
    };

    // convert database-output
    for mapping in mappings {
        let obj = ProjectMemberResp {
            user_id: mapping.user_id,
            project_role: mapping.role,
        };

        resp.members.push(obj);
    }

    Ok(Json(resp))
}
