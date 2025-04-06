// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

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

use crate::api::errors::ErrorResponse;
use crate::api::token_handling;
use crate::database::user_table;
use crate::common::functions::sha256_hash;

use super::auth_structs::{UserLoginReq, UserLoginResp};

#[api_operation(
    tag = "auth",
    summary = "Create Token",
    description = r###"Create Token for given user credentials"###,
    error_code = 401,
    error_code = 500
)]
pub async fn create_token(body: Json<UserLoginReq>) -> Result<CreatedJson<UserLoginResp>, ErrorResponse> {
    // get user from database
    let user: user_table::User;
    match user_table::get_user(&body.id) {
        Ok(val) => user = val,
        Err(_) => {
            return Err(ErrorResponse::Unauthorized("Invalid user-id or passphrase".to_string()));
        }
    }

    // check passphrase
    let salted_passphrase = format!("{}{}", body.passphrase, user.salt);
    let pw_hash = sha256_hash(salted_passphrase.as_str());
    if pw_hash != user.pw_hash {
        return Err(ErrorResponse::Unauthorized("Invalid user-id or passphrase".to_string()));
    }

    // create token based for the user
    match token_handling::create_token(
        &user.id, 
        &"".to_string(), 
        user.is_admin, 
        false)
    {
        Ok(token) => {
            let response = UserLoginResp{
                token: token,
            };
        
            return Ok(CreatedJson(response));
        },
        Err(_) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }
}
