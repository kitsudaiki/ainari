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

use crate::api::token_handling;
use crate::config;
use crate::database::user_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::auth_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "auth",
    summary = "Renew Token",
    description = r###"Create a new access-token for the user of the current token, in order to extend the session.

With `project_id` in the body, the new token is created for this project instead of the project of the current token, which switches the project without a new login. The user must be assigned to the project. The role within the project is read again in both cases, so changes of the role are applied with the renewal."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn renew_token(
    body: Json<TokenRenewReq>,
    context: UserContext,
) -> Result<CreatedJson<UserTokenResp>, ErrorResponse> {
    // validate request
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // the user could have been deleted, since the current token was created
    let user = user_table::get_auth_user(&context.user_id)
        .map_err(|_| ErrorResponse::Unauthorized("User doesn't exist anymore".to_string()))?;

    // use the requested project, or stay in the project of the current token
    let project_id = body
        .into_inner()
        .project_id
        .unwrap_or_else(|| context.project_id.clone());

    // get the current role of the user within the project, which also checks the access to it
    let project_role = super::get_project_role_for_token(&user.id, &project_id)?;

    let token =
        token_handling::create_token(&user.id, &project_id, &user.is_admin, project_role.as_str())
            .map_err(|_| ErrorResponse::InternalError("Internal Error".to_string()))?;

    let response = UserTokenResp {
        access_token: token,
        token_type: "bearer".to_string(),
        expires: config::CONFIG.auth.token_expire_time,
    };

    Ok(CreatedJson(response))
}
