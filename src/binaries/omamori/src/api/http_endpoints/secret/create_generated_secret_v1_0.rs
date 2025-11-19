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
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::RngCore;
use uuid::Uuid;
use validator::Validate; // needed to use .encode() and .decode()

use crate::core::crypto_trait::CryptoModule;
use crate::core::simple_crypto::SimpleCrypto;
use crate::database::secret_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::secret_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

#[api_operation(
    tag = "secret",
    summary = "Create new secret",
    description = r###"Create new secret based on a secret-template."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_secret(
    body: Json<SecretGenerateReq>,
    context: UserContext,
) -> Result<CreatedJson<SecretResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    super::check_quota(&context).await?;

    let secret_uuid = Uuid::new_v4();

    // generate key
    let b64 = generate_256bit_key_base64();

    // encrypt the secret with the simple-crypto-module
    let simple_crypto = SimpleCrypto::new();
    match simple_crypto.store(&secret_uuid, &b64) {
        Ok(()) => {}
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

    // add new secret to datbase
    match secret_table::add_new_secret(&secret_uuid, &body.name, &context) {
        Ok(_) => {}
        Err(e) => {
            log::error!("Failed to add secret with UUID '{secret_uuid}' to database.: {e}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // get new created secret from database to get addtional information
    match secret_table::get_secret(&secret_uuid, &context) {
        Ok(secret) => {
            let resp = SecretResp {
                uuid: secret_uuid,
                name: secret.name.clone(),
                created_by: secret.created_by.clone(),
                created_at: secret.created_at.clone(),
                updated_by: secret.updated_by.clone(),
                updated_at: secret.updated_at.clone(),
            };

            return Ok(CreatedJson(resp));
        }
        Err(_) => {
            log::error!(
                "Failed to get secret with UUID '{secret_uuid}' from database, even the secret should exist"
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };
}

fn generate_256bit_key_base64() -> Secret {
    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);
    Secret::from(STANDARD.encode(key))
}
