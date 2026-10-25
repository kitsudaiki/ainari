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

use crate::database::project_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::auth_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "auth",
    summary = "Validate Token",
    description = r###"Validate the access-token of the request.

The check itself is done by the middleware, so a successful response means that
the token is valid, and the content of the token is returned.

`project_locked` is true, while the project of the token is locked by a backup or restore. The
other components only allow reading requests of the user to a locked project."###,
    error_code = 401,
    error_code = 500
)]
pub async fn validate_token(
    context: UserContext,
) -> Result<Json<UserTokenValidateResp>, ErrorResponse> {
    // the lock is read from the database for every validation, because it changes, while the
    // token is valid
    let project_locked = match project_table::is_project_locked(&context.project_id) {
        Ok(project_locked) => project_locked,
        Err(ainari_common::enums::DbError::NotFound) => {
            return Err(ErrorResponse::Unauthorized(format!(
                "Project '{}' of the token doesn't exist anymore.",
                context.project_id
            )));
        }
        Err(_) => {
            ainari_api::report_internal_error!(
                Some(&context),
                "Failed to read lock of project '{}' from database",
                context.project_id
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let response = UserTokenValidateResp {
        context,
        project_locked,
    };
    // HINT(kitsudaki): Here is not validation-code, even the function is named this way,
    // because it provides only the endpoint itself. The token-validation will be done
    // in the middleware, like for the other endpoints
    Ok(Json(response))
}
