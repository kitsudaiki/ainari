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
use uuid::Uuid;
use validator::Validate;

use crate::core::crypto_trait::CryptoModule;
use crate::core::simple_crypto::SimpleCrypto;
use crate::database::secret_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::secret_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "secret",
    summary = "Clone secret (internal)",
    description = r###"Create a new secret as copy of an existing secret with the same payload,

which is owned by the given resource."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn clone_secret_internal(
    body: Json<SecretCloneInternalReq>,
    context: UserContext,
) -> Result<CreatedJson<SecretResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    super::check_quota(&context).await?;

    // check if the source-secret exist and is accessible with the given context
    let source_uuid = body.secret_uuid;
    let source_secret = secret_table::get_secret(&source_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("secret", &source_uuid, e))?;

    // only user-owned secrets can be cloned
    if source_secret.owned_by != "user" {
        return Err(ErrorResponse::Unauthorized(
            "Secret is owned by a resource and can not be cloned".to_string(),
        ));
    }

    let secret_uuid = Uuid::new_v4();

    // copy the payload of the source-secret within the simple-crypto-module
    let simple_crypto = SimpleCrypto::new();
    simple_crypto
        .clone_payload(&source_uuid, &secret_uuid)
        .map_err(map_ainari_error_to_api_response)?;

    // add new secret to database
    if let Err(e) = secret_table::add_new_secret(
        &secret_uuid,
        &source_secret.name,
        &body.owned_by,
        Some(body.resource_uuid),
        &context,
    ) {
        // remove the already copied payload again, so it doesn't stay without secret
        let _ = simple_crypto.delete(&secret_uuid);
        return Err(map_db_write_error(
            &format!("add secret with UUID '{secret_uuid}' to database"),
            e,
        ));
    }

    // get new created secret from database to get additional information
    let secret = secret_table::get_secret(&secret_uuid, &context)
        .map_err(|e| map_db_uuid_get_after_add_error("secret", &secret_uuid, e))?;

    let resp = SecretResp {
        uuid: secret_uuid,
        name: secret.name,
        owned_by: secret.owned_by,
        resource_uuid: secret.resource_uuid,
        created_by: secret.created_by,
        created_at: secret.created_at,
        updated_by: secret.updated_by,
        updated_at: secret.updated_at,
    };

    Ok(CreatedJson(resp))
}
