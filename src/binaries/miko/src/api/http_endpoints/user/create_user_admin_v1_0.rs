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

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::user_structs::*;
use ainari_common::enums;

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
    if !context.is_admin {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }

    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    let id = &body.id;

    // check if user already exist within the database
    match user_table::get_user(id, &context) {
        Ok(_) => {
            let msg = format!("User with ID '{id}' already exist.");
            return Err(ErrorResponse::Conflict(msg));
        }
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            // it is desired, that the user not already exist, so this error will be ignored
        }
    };

    // add new quota for the user to datbase
    match quota_table::add_new_quota(id, 10, 10, 10, 10, 10, &context) {
        Ok(_) => {}
        Err(e) => {
            log::error!("Failed to add quota for user with ID '{id}' to database.: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // add new user to datbase
    match user_table::add_new_user(id, &body.name, &body.passphrase, body.is_admin, &context) {
        Ok(_) => {}
        Err(e) => {
            // delete quota again, if adding of the user failed, to avoid inconsistent database
            quota_table::hard_delete_quota(id, &context);
            log::error!("Failed to add user with ID '{id}' to database.: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created user from database to get addtional information
    match user_table::get_user(id, &context) {
        Ok(user) => {
            let resp = UserResp {
                id: user.id.clone(),
                name: user.name.clone(),
                is_admin: body.is_admin,
                created_by: user.created_by.clone(),
                created_at: user.created_at.clone(),
                updated_by: user.updated_by.clone(),
                updated_at: user.updated_at.clone(),
            };

            return Ok(CreatedJson(resp));
        }
        Err(_) => {
            log::error!(
                "Failed to get user with ID '{id}' from database, even the user should exist"
            );
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
}
