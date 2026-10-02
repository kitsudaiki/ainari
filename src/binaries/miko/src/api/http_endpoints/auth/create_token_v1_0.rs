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
use crate::database::project_table;
use crate::database::user_project_mapping_table;
use crate::database::user_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::auth_structs::*;
use ainari_common::enums::DbError;
use ainari_common::functions::sha256_hash;

#[api_operation(
    tag = "auth",
    summary = "Create Token",
    description = r###"Create a new access-token for the given user-credentials. The token is scoped to the project given by the optional `project_id`, or to the default-project of the user, if not set. The user must be assigned to the project."###,
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
    let salted_passphrase = format!("{}{}", parsed.client_secret, user.salt);
    let pw_hash = sha256_hash(salted_passphrase.as_str());
    if pw_hash != user.pw_hash {
        return Err(ErrorResponse::Unauthorized(
            "Invalid user-id or passphrase".to_string(),
        ));
    }

    // use the requested project, or the default-project of the user, if none was requested
    let project_id = parsed
        .project_id
        .unwrap_or_else(|| format!("default-{}", user.id));

    // check if the project exist. A missing project and a missing mapping give the same error, so
    // the response doesn't reveal, which projects exist.
    let no_access_msg = format!("User has no access to project '{project_id}'");
    project_table::get_auth_project(&project_id).map_err(|e| match e {
        DbError::NotFound => ErrorResponse::Unauthorized(no_access_msg.clone()),
        DbError::InternalError => ErrorResponse::InternalError("Internal Error".to_string()),
    })?;

    // get the role of the user within the project
    let mapping =
        user_project_mapping_table::get_mapping(&project_id, &user.id).map_err(|e| match e {
            DbError::NotFound => ErrorResponse::Unauthorized(no_access_msg.clone()),
            DbError::InternalError => ErrorResponse::InternalError("Internal Error".to_string()),
        })?;

    // create token based for the user
    let token =
        token_handling::create_token(&user.id, &project_id, &user.is_admin, mapping.role.as_str())
            .map_err(|_| ErrorResponse::InternalError("Internal Error".to_string()))?;

    let response = UserTokenResp {
        access_token: token,
        token_type: "bearer".to_string(),
        expires: config::CONFIG.auth.token_expire_time,
    };

    Ok(Json(response))
}

/// Parses the form-encoded body of a token-request.
///
/// The token-endpoint follows the oauth2-conventions, so the credentials arrive as
/// `application/x-www-form-urlencoded` instead of json.
///
/// # Arguments
///
/// * `body` - The raw request-body
///
/// # Returns
///
/// * `Ok(OAuth2Request)` - The parsed request.
/// * `Err(...)` - The body is not valid form-encoding or misses a required field.
fn parse_oauth2_body(body: &str) -> Result<OAuth2Request, serde_urlencoded::de::Error> {
    serde_urlencoded::from_str(body)
}
