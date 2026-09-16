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
use validator::Validate;

use crate::core::crypto::apply_connection_policies;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::get_local_ip;
use crate::core::models::Connection;

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_crypto_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_crypto",
    summary = "Toggle crypto",
    description = r###"Switch the encryption of a single VM-to-VM connection on or off.

This makes the protection optional per connection without losing anything: the
keys of the connection stay installed in the kernel, only the policies are
rewritten. A disabled connection sends and accepts plain traffic even though a
key is set for it, and enabling it again immediately puts the keys back to use.

Both gateways of a connection have to be switched, otherwise the side that still
demands ESP drops the now unprotected packets of the other one."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn toggle_crypto_internal(
    body: Json<CryptoToggleReq>,
    _context: UserContext,
) -> Result<Json<CryptoToggleResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let local_gateway_ip = match get_local_ip("eth0") {
        Some(ip) => ip,
        None => {
            log::error!("No underlay address on eth0");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let conn_id = format!("{}->{}", body.local_ip, body.remote_ip);
    let mut st = GATEWAY_STATE_HANDLE.lock().await;

    // A connection this gateway holds a key for is already known. For an unknown
    // one the peer has to be named, otherwise there is no tunnel to describe.
    let mut conn = match st.connections.get(&conn_id).cloned() {
        Some(conn) => conn,
        None => {
            let peer = match body.peer_gateway_ip {
                Some(peer) => peer,
                None => {
                    return Err(ErrorResponse::NotFound(
                        "Unknown connection. Install a key first or pass peer_gateway_ip."
                            .to_string(),
                    ));
                }
            };
            Connection {
                local_ip: body.local_ip,
                remote_ip: body.remote_ip,
                peer_gateway_ip: peer,
                enabled: body.enabled,
                active_egress_spi: None,
            }
        }
    };

    conn.enabled = body.enabled;
    if let Some(peer) = body.peer_gateway_ip {
        conn.peer_gateway_ip = peer;
    }

    apply_connection_policies(&conn, local_gateway_ip)
        .map_err(|e| map_internal_error("apply connection-policies", e))?;

    let keys_held = st
        .crypto_keys
        .values()
        .filter(|key| key.local_ip == conn.local_ip && key.remote_ip == conn.remote_ip)
        .count();
    st.connections.insert(conn_id, conn);

    let message = if body.enabled {
        format!(
            "Encryption enabled for {} -> {} ({} key(s) available)",
            body.local_ip, body.remote_ip, keys_held
        )
    } else {
        format!(
            "Encryption disabled for {} -> {}; {} key(s) stay installed but unused",
            body.local_ip, body.remote_ip, keys_held
        )
    };
    log::debug!("{}", message);

    let resp = CryptoToggleResp {
        local_ip: body.local_ip.clone(),
        remote_ip: body.remote_ip.clone(),
        peer_gateway_ip: body.peer_gateway_ip.clone(),
        enabled: body.enabled.clone(),
    };

    Ok(Json(resp))
}
