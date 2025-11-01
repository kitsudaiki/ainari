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
use uuid::Uuid;

use crate::database::secret_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::secret_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "secret",
    summary = "Get secret",
    description = r###"Get information of a secret from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_secret(
    secret_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<SecretResp>, ErrorResponse> {
    let secret_data = match secret_table::get_secret(&secret_uuid, &context) {
        Ok(secret_data) => secret_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Secret with UUID '{secret_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let resp = SecretResp {
        uuid: *secret_uuid,
        name: secret_data.name.clone(),
        created_by: secret_data.created_by.clone(),
        created_at: secret_data.created_at.clone(),
        updated_by: secret_data.updated_by.clone(),
        updated_at: secret_data.updated_at.clone(),
    };

    return Ok(Json(resp));
}
