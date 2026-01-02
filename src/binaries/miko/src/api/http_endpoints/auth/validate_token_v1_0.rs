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

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::auth_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "auth",
    summary = "Validate Token",
    description = r###"Validate Token"###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn validate_token(
    context: UserContext,
) -> Result<Json<UserTokenValidateResp>, ErrorResponse> {
    let response = UserTokenValidateResp { context };
    // HINT(kitsudaki): Here is not validation-code, even the funktion is named this way,
    // because it provides only the endpoint itself. The token-validation will be done
    // in the middleway, like for the other endpoints
    Ok(Json(response))
}
