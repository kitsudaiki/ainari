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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;

use crate::database::quota_table;
use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "user",
    summary = "Delete user",
    description = r###"Delete a user from the database. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_user_admin(
    user_id: Path<String>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    check_admin_context(&context)?;

    if context.user_id == user_id.to_string() {
        return Err(ErrorResponse::Conflict(
            "A user can not delete himself.".to_string(),
        ));
    }

    // delete quota of user from database
    quota_table::delete_quota(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("quota", &user_id, e))?;

    // delete user from database
    user_table::delete_user(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", &user_id, e))?;

    Ok(NoContent)
}
