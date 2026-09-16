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

use crate::core::crypto::{apply_connection_policies, install_sa, normalize_key};
use crate::core::models::{Connection, CryptoKey};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::get_local_ip;

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_crypto_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_crypto",
    summary = "Register new crypto-key",
    description = r###"Install one AES-256-GCM key for one direction of a VM-to-VM connection.

A key is programmed as an xfrm Security Association in tunnel mode between the
two gateways, with a selector that pins it to exactly one VM pair. That is what
gives every connection its own key material even though several VMs may share
the same remote gateway.

`egress` creates the outbound SA and points the outbound policy of the VM pair at
its SPI, which makes it the key used for new packets, so a rotation never has a
gap in which traffic would leave unprotected. `ingress` creates an inbound SA and
the matching in/fwd policies; any number of inbound keys can be held for the same
connection at the same time - the SPI in the ESP header of an incoming packet
decides which one is used."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn register_crypto_key_internal(
    body: Json<CryptoKeyReq>,
    _context: UserContext,
) -> Result<CreatedJson<CryptoKeyResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let key = normalize_key(&body.key).map_err(ErrorResponse::BadRequest)?;

    let local_gateway_ip = match get_local_ip("eth0") {
        Some(ip) => ip,
        None => {
            log::error!("No underlay address on eth0");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let spi = format!("0x{:08x}", body.spi);
    let local_sel = format!("{}/32", body.local_ip);
    let remote_sel = format!("{}/32", body.remote_ip);
    let conn_id = format!("{}->{}", body.local_ip, body.remote_ip);

    let mut st = GATEWAY_STATE_HANDLE.lock().await;

    // Remember the connection this key belongs to. A connection is protected as
    // soon as it exists; the toggle endpoint can switch that off again without
    // touching the keys.
    let mut conn = st.connections.get(&conn_id).cloned().unwrap_or(Connection {
        local_ip: body.local_ip,
        remote_ip: body.remote_ip,
        peer_gateway_ip: body.peer_gateway_ip,
        enabled: true,
        active_egress_spi: None,
    });
    conn.peer_gateway_ip = body.peer_gateway_ip;

    // The SA always describes the tunnel between the two gateways; only its
    // direction and its selector differ.
    let result = match body.direction.as_str() {
        "egress" => install_sa(
            local_gateway_ip,
            body.peer_gateway_ip,
            &spi,
            &key,
            &local_sel,
            &remote_sel,
        )
        .map(|()| {
            // The newest outbound key becomes the active one, which the policy
            // is pinned to further down.
            conn.active_egress_spi = Some(body.spi);
        }),
        "ingress" => install_sa(
            body.peer_gateway_ip,
            local_gateway_ip,
            &spi,
            &key,
            &remote_sel,
            &local_sel,
        ),
        other => {
            return Err(ErrorResponse::BadRequest(format!(
                "Unknown direction '{}', expected egress or ingress",
                other
            )));
        }
    };

    result.map_err(|e| map_internal_error("install crypto-key", e))?;

    // Write the policies of the connection. They demand ESP while the encryption
    // is switched on and are plain allow rules while it is switched off, so a
    // key installed on a disabled connection is stored but stays unused.
    apply_connection_policies(&conn, local_gateway_ip)
        .map_err(|e| map_internal_error("apply connection-policies", e))?;

    let encryption_state = if conn.enabled {
        "active"
    } else {
        "stored, encryption disabled"
    };
    st.connections.insert(conn_id, conn);

    let entry = CryptoKey {
        direction: body.direction.clone(),
        local_ip: body.local_ip,
        remote_ip: body.remote_ip,
        peer_gateway_ip: body.peer_gateway_ip,
        spi: body.spi,
    };
    st.crypto_keys
        .insert(format!("{}:{}", body.direction, body.spi), entry);

    log::debug!(
        "Installed {} key spi 0x{:08x} for {} <-> {} via {} ({})",
        body.direction,
        body.spi,
        body.local_ip,
        body.remote_ip,
        body.peer_gateway_ip,
        encryption_state
    );

    let resp = CryptoKeyResp {
        direction: body.direction.clone(),
        local_ip: body.local_ip,
        remote_ip: body.remote_ip,
        peer_gateway_ip: body.peer_gateway_ip,
        spi: body.spi,
    };

    Ok(CreatedJson(resp))
}
