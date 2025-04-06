// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use apistos::actix::NoContent;
use apistos::api_operation;
use actix_web::web::Path;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::user_table;
use crate::common::enums;

#[api_operation(
    tag = "user",
    summary = "Delete user",
    description = r###"Delete a user from the database. This can only be done by an admin."###,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_user(id: Path<String>, context: UserContext) -> Result<NoContent, ErrorResponse> {
    if context.is_admin == false {
        return Err(ErrorResponse::Unauthorized("Only Admins are allowed to use this endpoint".to_string()));
    }

    // get new created user from database to get addtional information
    match user_table::delete_user(&id) {
        Ok(_) => {
            return Ok(NoContent);
        },
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };
}