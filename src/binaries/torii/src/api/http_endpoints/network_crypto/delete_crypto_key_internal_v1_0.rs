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

use crate::config::CONFIG;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::{get_local_ip, run_ip};

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_crypto_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_crypto",
    summary = "Delete crypto-key",
    description = r###"Remove one IPsec key again, which is the second half of a key rotation.

Only the Security Association is dropped; the policies of the connection stay in
place. Deleting the key a policy currently points at therefore does not open a
hole - the traffic is discarded until another key is installed."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_crypto_key_internal(
    path: Path<CryptoKeyPath>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let CryptoKeyPath { direction, spi } = path.into_inner();
    let mut st = GATEWAY_STATE_HANDLE.lock().await;

    let entry = match st.crypto_keys.remove(&(direction, spi)) {
        Some(entry) => entry,
        None => return Err(ErrorResponse::NotFound("No such key".to_string())),
    };

    let local_gateway_ip = match get_local_ip(&CONFIG.network.underlay_iface) {
        Some(ip) => ip,
        None => {
            log::error!("No underlay address on {}", CONFIG.network.underlay_iface);
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let (src, dst) = match entry.direction {
        CryptoDirection::Egress => (local_gateway_ip, entry.peer_gateway_ip),
        CryptoDirection::Ingress => (entry.peer_gateway_ip, local_gateway_ip),
    };
    let (src, dst) = (src.to_string(), dst.to_string());
    let spi_hex = format!("0x{:08x}", spi);

    run_ip(&[
        "xfrm", "state", "delete", "src", &src, "dst", &dst, "proto", "esp", "spi", &spi_hex,
    ])
    .map_err(|e| map_internal_error(&format!("delete crypto-key with spi '{spi_hex}'"), e))?;

    println!(
        "Removed {} key spi {} for {} <-> {}",
        entry.direction, spi_hex, entry.local_ip, entry.remote_ip
    );

    Ok(NoContent)
}
