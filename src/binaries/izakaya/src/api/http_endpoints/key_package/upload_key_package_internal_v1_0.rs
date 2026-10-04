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
use validator::Validate;

use crate::core::key_package::validate_key_package;
use crate::database::key_package_table;

use ainari_api::common_functions::map_db_write_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "key_package",
    summary = "Upload key-packages",
    description = r###"Upload MLS key-packages of a gateway, so other gateways can invite it into their groups.

Every key-package is validated before it is stored: it has to be correctly signed, must not be
expired and the identity of its credential has to be the `client_id` of the request. If one of
them is invalid, none of them is stored."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 500
)]
pub async fn upload_key_package_internal(
    body: Json<KeyPackageUploadReq>,
    context: UserContext,
) -> Result<CreatedJson<KeyPackageUploadResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    for (index, key_package) in body.key_packages.iter().enumerate() {
        validate_key_package(&body.client_id, key_package).map_err(|e| {
            ErrorResponse::BadRequest(format!("Invalid key-package at position {index}: {e}"))
        })?;
    }

    let uuids = key_package_table::add_key_packages(&body.client_id, &body.key_packages, &context)
        .map_err(|e| {
            map_db_write_error(
                &format!("add key-packages of client '{}'", body.client_id),
                e,
            )
        })?;

    log::debug!(
        "Stored {} key-package(s) of client '{}'",
        uuids.len(),
        body.client_id
    );

    Ok(CreatedJson(KeyPackageUploadResp {
        client_id: body.client_id.clone(),
        uuids,
    }))
}
