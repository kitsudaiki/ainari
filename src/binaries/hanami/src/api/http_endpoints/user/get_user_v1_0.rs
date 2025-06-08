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
use actix_web::web::Path;
use apistos::api_operation;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::user_table;

use hanami_common::enums;
use hanami_structs::user_structs::UserResp;

#[api_operation(
    tag = "user",
    summary = "Get user",
    description = r###"Get information of a user from the database. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_user(user_id: Path<String>, context: UserContext) -> Result<Json<UserResp>, ErrorResponse> {
    if context.is_admin == false {
        return Err(ErrorResponse::Unauthorized("Only Admins are allowed to use this endpoint".to_string()));
    }

    // get new created user from database to get addtional information
    match user_table::get_user(&user_id, &context) {
        Ok(user) => {
            let resp = UserResp {
                id: user.id.clone(),
                name: user.name.clone(),
                is_admin: user.is_admin,
                created_by: user.created_by.clone(),
                created_at: user.created_at.clone(),
                updated_by: user.updated_by.clone(),
                updated_at: user.updated_at.clone(),
            };
        
            return Ok(Json(resp));
        },
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("User with ID '{user_id}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };
}