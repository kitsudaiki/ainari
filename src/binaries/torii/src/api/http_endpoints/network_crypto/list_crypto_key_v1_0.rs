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

use crate::core::models::CryptoKey;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_crypto_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_crypto",
    summary = "List crypto-keys",
    description = r###"List the IPsec keys this gateway currently holds.

The key material is never part of the answer - it is written straight into the
kernel and not kept in the application state."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_crypto_key(
    _context: UserContext,
) -> Result<Json<CryptoKeyListResp>, ErrorResponse> {
    let st = GATEWAY_STATE_HANDLE.lock().await;

    let mut crypto_keys: Vec<CryptoKey> = st.crypto_keys.values().cloned().collect();
    crypto_keys.sort_by_key(|key| (key.direction.clone(), key.spi));

    let mut resp = CryptoKeyListResp::default();
    for crypto_key in crypto_keys {
        let converted_crypto_key = CryptoKeyResp {
            direction: crypto_key.direction,
            local_ip: crypto_key.local_ip,
            remote_ip: crypto_key.remote_ip,
            peer_gateway_ip: crypto_key.peer_gateway_ip,
            spi: crypto_key.spi,
        };

        resp.keys.push(converted_crypto_key);
    }

    Ok(Json(resp))
}
