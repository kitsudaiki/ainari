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
use apistos::api_operation;

use crate::core::mls_key_exchange::state::MLS_STATE_HANDLE;
use crate::core::mls_key_exchange::status;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_crypto_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_crypto",
    summary = "Get state of the MLS key-exchange",
    description = r###"Get the state of the key-exchange of this gateway.

Every network lists the pairs of this gateway with the other gateways of the network. A pair
moves to the keys of a new epoch on its own, only coordinated with its peer, so the list tells for
every pair its epochs and the step, which it waits for."###,
    error_code = 401,
    error_code = 500
)]
pub async fn get_mls_status_internal(
    _context: UserContext,
) -> Result<Json<MlsStatusResp>, ErrorResponse> {
    let mls = MLS_STATE_HANDLE.lock().await;
    Ok(Json(status::status(&mls)))
}
