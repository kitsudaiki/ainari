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

use actix_web::web::Json;
use apistos::api_operation;

use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::user_structs::*;

#[api_operation(
    tag = "user",
    summary = "List user",
    description = r###"List basic information of all user from the database. This can only be done by an admin."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_user_admin(context: UserContext) -> Result<Json<UserListResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;

    let users = user_table::list_users(&context).map_err(|e| map_db_list_error("users", e))?;

    let mut resp = UserListResp { users: Vec::new() };

    for user in users {
        let obj = UserBasicResp {
            id: user.id.clone(),
            name: user.name.clone(),
            is_admin: user.is_admin,
        };

        resp.users.push(obj);
    }

    Ok(Json(resp))
}
