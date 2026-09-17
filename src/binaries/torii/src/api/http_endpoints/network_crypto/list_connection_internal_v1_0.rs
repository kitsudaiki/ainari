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

use crate::core::models::Connection;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_crypto_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_crypto",
    summary = "List connections",
    description = r###"List the VM-to-VM connections this gateway knows about.

Shows for every connection whether its encryption is currently switched on and
which outbound key is in use, which is the quickest way to tell a protected
connection from a deliberately unprotected one."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_connection_internal(
    _context: UserContext,
) -> Result<Json<ConnectionListResp>, ErrorResponse> {
    let st = GATEWAY_STATE_HANDLE.lock().await;

    let mut connections: Vec<Connection> = st.connections.values().cloned().collect();
    connections.sort_by_key(|conn| (conn.local_ip, conn.remote_ip));

    let mut resp = ConnectionListResp::default();
    for connection in connections {
        let converted_route = ConnectionResp {
            local_ip: connection.local_ip,
            remote_ip: connection.remote_ip,
            peer_gateway_ip: connection.peer_gateway_ip,
            enabled: connection.enabled,
            active_egress_spi: connection.active_egress_spi,
        };

        resp.connections.push(converted_route);
    }

    Ok(Json(resp))
}
