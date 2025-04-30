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

use apistos::api_operation;
use apistos::actix::CreatedJson;

use crate::api::token_handling;
use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;

use super::auth_structs::UserTokenResp;

#[api_operation(
    tag = "auth",
    summary = "Renew Token",
    description = r###"Renew Token"###,
    error_code = 401,
    error_code = 500
)]
pub async fn renew_token(context: UserContext) -> Result<CreatedJson<UserTokenResp>, ErrorResponse> {
    let token = match token_handling::create_token(
        &context.user_id, 
        &context.project_id, 
        context.is_admin, 
        context.is_project_admin)
    {
        Ok(token) => token,
        Err(_) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let response = UserTokenResp{
        access_token: token,
        token_type: "bearer".to_string(),
        expires: 3600,
    };

    return Ok(CreatedJson(response));
}