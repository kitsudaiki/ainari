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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::TryRng;
use uuid::Uuid;
use validator::Validate; // needed to use .encode() and .decode()

use crate::core::crypto_trait::CryptoModule;
use crate::core::simple_crypto::SimpleCrypto;
use crate::database::secret_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::secret_structs::*;
use ainari_api_structs::user_context::UserContext;
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
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    super::check_quota(&context).await?;

    let secret_uuid = Uuid::new_v4();

    // generate key
    let b64 = generate_256bit_key_base64();

    // encrypt the secret with the simple-crypto-module
    let simple_crypto = SimpleCrypto::new();
    simple_crypto
        .store(&secret_uuid, &b64)
        .map_err(map_ainari_error_to_api_response)?;

    // add new secret to datbase
    secret_table::add_new_secret(&secret_uuid, &body.name, &context).map_err(|e| {
        log::error!("Failed to add secret with UUID '{secret_uuid}' to database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // get new created secret from database to get addtional information
    let secret = secret_table::get_secret(&secret_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("project", &secret_uuid, e))?;

    let resp = SecretResp {
        uuid: secret_uuid,
        name: secret.name,
        created_by: secret.created_by,
        created_at: secret.created_at,
        updated_by: secret.updated_by,
        updated_at: secret.updated_at,
    };

    Ok(CreatedJson(resp))
}

fn generate_256bit_key_base64() -> Secret {
    let mut key = [0u8; 32];
    let _ = rand::rng().try_fill_bytes(&mut key);
    Secret::from(STANDARD.encode(key))
}
