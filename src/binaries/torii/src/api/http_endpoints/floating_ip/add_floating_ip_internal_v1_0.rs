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
use uuid::Uuid;
use validator::Validate;

use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "floating_ip",
    summary = "Register new floating-ip",
    description = r###"Provision a floating IP and register its SNAT/DNAT rules in eBPF.

The floating IP is associated with a private internal IP, which updates both the
`FIP_DNAT_MAP` and the `FIP_SNAT_MAP` of the datapath."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn register_floating_ip_internal(
    body: Json<FloatingIpInternalCreateReq>,
    _context: UserContext,
) -> Result<CreatedJson<FloatingIpInternalResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let uuid = Uuid::new_v4();
    let mut state = GATEWAY_STATE_HANDLE.lock().await;

    state.floating_ips
        .insert(body.floating_ip.clone(), body.internal_ip.clone());

    if state
        .fip_dnat_map
        .insert(u32::from(body.floating_ip), u32::from(body.internal_ip), 0)
        .is_err()
    {
        return Err(ErrorResponse::InternalError(
            "eBPF Map error (DNAT)".to_string(),
        ));
    }
    if state
        .fip_snat_map
        .insert(u32::from(body.internal_ip), u32::from(body.floating_ip), 0)
        .is_err()
    {
        return Err(ErrorResponse::InternalError(
            "eBPF Map error (SNAT)".to_string(),
        ));
    }

    let resp = FloatingIpInternalResp {
        uuid: uuid,
        name: body.name.clone(),
        network_uuid: body.network_uuid.clone(),
        floating_ip: body.floating_ip.clone(),
        internal_ip: body.internal_ip.clone(),
    };

    Ok(CreatedJson(resp))
}
