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
use validator::Validate;

use crate::api::token_handling;
use crate::config;
use crate::database::user_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::auth_structs::*;
use ainari_common::functions::sha256_hash;

#[api_operation(
    tag = "auth",
    summary = "Create Token",
    description = r###"Create Token for given user credentials"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_token(body: String) -> Result<Json<UserTokenResp>, ErrorResponse> {
    let parsed = parse_oauth2_body(body.as_str())
        .map_err(|e| ErrorResponse::BadRequest(format!("Failed to parse body: {e}")))?;

    // validate incoming json
    parsed
        .validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // get and check token-format
    if parsed.token_format != "jwt" {
        let token_format = parsed.token_format;
        let msg =
            format!("Token-format '{token_format}' is not supported. Supported formats: [ jwt ]");
        return Err(ErrorResponse::BadRequest(msg));
    }

    // get and check grant-type
    if parsed.grant_type != "client_credentials" {
        let grant_type = parsed.grant_type;
        let msg = format!(
            "Grant-type '{grant_type}' is not supported. Supported types: [ client_credentials ]"
        );
        return Err(ErrorResponse::BadRequest(msg));
    }

    // get user from database
    let user = user_table::get_auth_user(&parsed.client_id)
        .map_err(|_| ErrorResponse::Unauthorized("Invalid user-id or passphrase".to_string()))?;

    // check passphrase
    let salted_passphrase = format!("{}{}", &parsed.client_secret, user.salt);
    let pw_hash = sha256_hash(salted_passphrase.as_str());
    if pw_hash != user.pw_hash {
        return Err(ErrorResponse::Unauthorized(
            "Invalid user-id or passphrase".to_string(),
        ));
    }

    // create token based for the user
    let token = token_handling::create_token(
        &user.id,
        &"".to_string(),
        &user.is_admin,
        &false.to_string(),
    )
    .map_err(|_| ErrorResponse::InternalError("Internal Error".to_string()))?;

    let response = UserTokenResp {
        access_token: token,
        token_type: "bearer".to_string(),
        expires: config::CONFIG.auth.token_expire_time,
    };

    Ok(Json(response))
}

fn parse_oauth2_body(body: &str) -> Result<OAuth2Request, serde_urlencoded::de::Error> {
    serde_urlencoded::from_str(body)
}
