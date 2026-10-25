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

use crate::config;
use crate::core::token_handling;
use crate::database::user_table;

use ainari_api::common_functions::check_admin_context;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::auth_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "auth",
    summary = "Renew Token for project (admin)",
    description = r###"Create a new access-token of an admin for any existing project, also if the admin is not assigned to the project.

Some components act on the project of the token, like isekai, when it restores a backup into another or a new project. The admin keeps the role within the project, if the admin is assigned to it, otherwise the token gets the role `admin`. The admin-flag of the user is read again from the database. This can only be done by an admin, the normal renewal of a token over `PUT /v1alpha/token` only allows projects, to which the user is assigned."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn renew_token_for_project_admin(
    body: Json<TokenProjectAdminReq>,
    context: UserContext,
) -> Result<CreatedJson<UserTokenResp>, ErrorResponse> {
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // the user could have been deleted or degraded, since the current token was created
    let user = user_table::get_auth_user(&context.user_id)
        .map_err(|_| ErrorResponse::Unauthorized("User doesn't exist anymore".to_string()))?;
    let project_id = body.into_inner().project_id;
    let project_role = super::get_admin_role_for_token(&user, &project_id)?;

    let token =
        token_handling::create_token(&user.id, &project_id, &user.is_admin, project_role.as_str())
            .map_err(|_| {
                ainari_api::report_internal_error!(
                    Some(&context),
                    "Failed to create token for user '{}' in project '{project_id}'",
                    user.id
                );
                ErrorResponse::InternalError("Internal Error".to_string())
            })?;

    Ok(CreatedJson(UserTokenResp {
        access_token: token,
        token_type: "bearer".to_string(),
        expires: config::CONFIG.auth.token_expire_time,
    }))
}
