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

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::secret_structs::*;
use ainari_api_structs::user_context::UserContext;

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
    // check if secret exist
    secret_table::get_secret(&secret_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("secret", &secret_uuid, e))?;

    // decrypt the secret with the simple-crypto-module
    let simple_crypto = SimpleCrypto::new();
    let plaintext = simple_crypto
        .retrieve(&secret_uuid)
        .map_err(map_ainari_error_to_api_response)?;

    let resp = SecretWithPayloadResp {
        secret_payload: plaintext.reveal().to_string(),
    };

    Ok(Json(resp))
}
