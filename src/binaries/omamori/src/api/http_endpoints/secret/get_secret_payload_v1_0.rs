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

use crate::core::crypto_trait::CryptoModule;
use crate::core::simple_crypto::SimpleCrypto;
use crate::database::secret_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::secret_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "secret",
    summary = "Get secret with payload",
    description = r###"Get information of a secret from the database with the payload."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_secret_with_payload(
    secret_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<SecretWithPayloadResp>, ErrorResponse> {
    let _ = match secret_table::get_secret(&secret_uuid, &context) {
        Ok(secret_data) => secret_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Secret with UUID '{secret_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // decrypt the secret with the simple-crypto-module
    let simple_crypto = SimpleCrypto::new();
    let plaintext = match simple_crypto.retrieve(&secret_uuid) {
        Ok(plaintext) => plaintext,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let resp = SecretWithPayloadResp {
        secret_payload: plaintext.reveal().to_string(),
    };

    return Ok(Json(resp));
}
