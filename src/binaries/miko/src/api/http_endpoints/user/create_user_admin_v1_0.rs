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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::database::quota_table;
use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::user_structs::*;

#[api_operation(
    tag = "user",
    summary = "Create new user",
    description = r###"Create new user. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn create_user_admin(
    body: Json<UserCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<UserResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let user_id = &body.id;

    // check if user-id already exist
    check_if_id_exist_in_db("user", user_id, user_table::get_user(user_id, &context))?;

    // add new quota for the user to datbase
    quota_table::add_new_quota(user_id, 10, 10, 10, 10, 10, &context).map_err(|e| {
        log::error!("Failed to add quota for user with ID '{user_id}' to database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // add new user to datbase
    user_table::add_new_user(
        user_id,
        &body.name,
        &body.passphrase,
        &body.is_admin,
        &context,
    )
    .map_err(|e| {
        // delete quota again, if adding of the user failed, to avoid inconsistent database
        quota_table::hard_delete_quota(user_id, &context);
        log::error!("Failed to add user with ID '{user_id}' to database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // get new created user from database to get addtional information
    let user = user_table::get_user(user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", user_id, e))?;

    let resp = UserResp {
        id: user.id,
        name: user.name,
        is_admin: user.is_admin,
        created_by: user.created_by,
        created_at: user.created_at,
        updated_by: user.updated_by,
        updated_at: user.updated_at,
    };

    Ok(CreatedJson(resp))
}
