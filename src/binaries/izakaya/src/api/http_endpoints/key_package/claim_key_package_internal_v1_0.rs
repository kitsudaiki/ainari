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

use actix_web::web::{Json, Path};
use apistos::api_operation;

use crate::core::key_package::signature_key_of;
use crate::database::key_package_table;

use ainari_api::common_functions::map_db_id_get_delete_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "key_package",
    summary = "Claim key-package",
    description = r###"Claim one MLS key-package of a gateway to invite it into a group.

A key-package can only be used for one invitation, so it is removed with the claim. The newest
key-package of the gateway is handed out first. If a signature-key is given, only a key-package
with exactly this key is handed out."###,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 500
)]
pub async fn claim_key_package_internal(
    path: Path<ClientPath>,
    body: Json<KeyPackageClaimReq>,
    context: UserContext,
) -> Result<Json<KeyPackageResp>, ErrorResponse> {
    let client_id = &path.client_id;
    let signature_key = body.signature_key.clone();

    let accept = |entry: &key_package_table::KeyPackageEntry| match &signature_key {
        Some(signature_key) => {
            signature_key_of(&entry.key_package).as_deref() == Some(signature_key.as_str())
        }
        None => true,
    };
    let entry = key_package_table::claim_key_package(client_id, accept, &context)
        .map_err(|e| map_db_id_get_delete_error("key-package of client", client_id, e))?;

    log::debug!(
        "Key-package '{}' of client '{client_id}' was claimed",
        entry.uuid
    );

    Ok(Json(KeyPackageResp {
        uuid: entry.uuid,
        client_id: entry.client_id,
        key_package: entry.key_package,
        created_at: entry.created_at,
    }))
}
