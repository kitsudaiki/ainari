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
    summary = "Change passphrase of user",
    description = r###"Set a new passphrase for any user for all future logins, without the need of the old passphrase. Already existing tokens of the user stay valid. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn change_passphrase_admin(
    body: Json<PassphraseChangeAdminReq>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let user_id = &body.user_id;

    // write new passphrase to database, which also checks, if the user exist
    user_table::update_passphrase(user_id, &body.new_passphrase, &context)
        .map_err(|e| map_db_id_get_delete_error("user", user_id, e))?;

    Ok(NoContent)
}
