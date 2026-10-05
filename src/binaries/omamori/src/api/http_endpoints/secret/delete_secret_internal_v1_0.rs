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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::core::crypto_trait::CryptoModule;
use crate::core::simple_crypto::SimpleCrypto;
use crate::database::secret_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "secret",
    summary = "Delete secret (internal)",
    description = r###"Delete a secret from the database and its payload from the crypto-module."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_secret_internal(
    secret_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // only resouce owned secrets can be deleted over the internal endpoint
    // user owned are created over the public api and so they can only be deleted there again
    let secret = secret_table::get_secret(&secret_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("secret", &secret_uuid, e))?;
    if secret.owned_by == "user" {
        return Err(ErrorResponse::Unauthorized(
            "Secret is owned by a user and can not be deleted over internal endpoints".to_string(),
        ));
    }

    // delete secret-metadata-from db
    secret_table::delete_secret(&secret_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("secret", &secret_uuid, e))?;

    // delete secret from crypto-module
    let simple_crypto = SimpleCrypto::new();
    simple_crypto
        .delete(&secret_uuid)
        .map_err(map_ainari_error_to_api_response)?;

    Ok(NoContent)
}
