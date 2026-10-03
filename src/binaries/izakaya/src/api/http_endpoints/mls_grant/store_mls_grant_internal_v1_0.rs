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
use apistos::actix::NoContent;
use apistos::api_operation;
use validator::Validate;

use crate::config::GRANT_PUBLIC_KEY;
use crate::core::coordinator;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_grant",
    summary = "Store membership-grant",
    description = r###"Store a membership-grant of hanami for a gateway and the group of a network.

Only a grant with a valid signature of hanami is accepted. A grant, which adds the gateway, allows
it to subscribe to the group. A grant, which removes it, drops its grant and removes it from the
group, which rotates the keys of the remaining members."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn store_mls_grant_internal(
    body: Json<MlsGrantReq>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    coordinator::store_grant(&body.grant, &GRANT_PUBLIC_KEY, coordinator::now())?;

    Ok(NoContent)
}
