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
use apistos::actix::NoContent;
use apistos::api_operation;
use validator::Validate;

use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::auth_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "auth",
    summary = "Change passphrase",
    description = r###"Change the passphrase of the user of the token for all future logins. The old passphrase has to be provided as well. Already existing tokens stay valid."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn change_passphrase(
    body: Json<PassphraseChangeReq>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate request
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // the user can only change its own passphrase, so the user-id always comes from the token
    let user_id = &context.user_id;

    // get user from database
    let user = user_table::get_auth_user(user_id)
        .map_err(|e| map_db_id_get_delete_error("user", user_id, e))?;

    // check old passphrase
    if !user_table::verify_passphrase(&user, &body.old_passphrase) {
        return Err(ErrorResponse::Unauthorized(
            "Invalid old passphrase".to_string(),
        ));
    }

    // write new passphrase to database
    user_table::update_passphrase(user_id, &body.new_passphrase, &context)
        .map_err(|e| map_db_id_get_delete_error("user", user_id, e))?;

    Ok(NoContent)
}
