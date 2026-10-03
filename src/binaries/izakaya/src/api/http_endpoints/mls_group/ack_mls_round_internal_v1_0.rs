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
use apistos::actix::NoContent;
use apistos::api_operation;
use validator::Validate;

use crate::core::coordinator;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "mls_group",
    summary = "Acknowledge MLS-round",
    description = r###"Acknowledge, that a gateway finished a phase of the key-rotation of a MLS-group.

The next phase only starts, once every member acknowledged the current one, so both ends of every
connection always hold the keys, which the other end sends with. An acknowledgement, which is not
needed anymore, is ignored."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn ack_mls_round_internal(
    path: Path<MlsGroupVniPath>,
    body: Json<MlsRoundAckReq>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    coordinator::round_ack(path.vni, &body, coordinator::now())?;

    Ok(NoContent)
}
