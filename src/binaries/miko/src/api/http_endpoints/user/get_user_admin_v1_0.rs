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
use actix_web::web::Path;
use apistos::api_operation;

use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::user_structs::*;

#[api_operation(
    tag = "user",
    summary = "Get user",
    description = r###"Get information of a user from the database. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_user_admin(
    user_id: Path<String>,
    context: UserContext,
) -> Result<Json<UserResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;

    // get user from database
    let user = user_table::get_user(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", &user_id, e))?;

    let resp = UserResp {
        id: user.id,
        name: user.name,
        is_admin: user.is_admin,
        created_by: user.created_by,
        created_at: user.created_at,
        updated_by: user.updated_by,
        updated_at: user.updated_at,
    };

    Ok(Json(resp))
}
