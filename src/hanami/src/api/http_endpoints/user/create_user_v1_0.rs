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

use apistos::actix::CreatedJson;
use actix_web::web::Json;
use apistos::api_operation;
use log::error;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::user_table;
use hanami_common::enums;

use super::user_structs::{UserCreateReq, UserResp};

#[api_operation(
    tag = "user",
    summary = "Create new user",
    description = r###"Create new user. This can only be done by an admin."###,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn create_user(body: Json<UserCreateReq>, context: UserContext) -> Result<CreatedJson<UserResp>, ErrorResponse> {
    if context.is_admin == false {
        return Err(ErrorResponse::Unauthorized("Only Admins are allowed to use this endpoint".to_string()));
    }   

    // check if user already exist within the database
    match user_table::get_user(&body.id, &context) {
        Ok(_) => {
            let msg = format!("User with ID '{}' already exist.", body.id);
            return Err(ErrorResponse::Conflict(msg));
        },
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            // it is desired, that the user not already exist, so this error will be ignored
        }
    };

    // add new user to datbase
    match user_table::add_new_user(&body.id, &body.name, &body.passphrase, body.is_admin, &context) {
        Ok(_) => {},
        Err(_) => {
            let msg = format!("Failed to add user with ID '{}' to database.", body.id);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created user from database to get addtional information
    match user_table::get_user(&body.id, &context) {
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
        },
        Err(_) => 
        {
            let msg = format!("Failed to get user with ID '{}' from database, even the user should exist.", body.id);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
}