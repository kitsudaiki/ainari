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
use torii_common::{FipTarget, RouteKey};
use uuid::Uuid;
use validator::Validate;

use crate::core::models::{FipTargetPod, FloatingIp, RouteKeyPod};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::validate_vni;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "floating_ip",
    summary = "Register new floating-ip",
    description = r###"Provision a floating IP and register its SNAT/DNAT rules in eBPF.

The floating IP is associated with a private internal IP, which updates both the
`FIP_DNAT_MAP` and the `FIP_SNAT_MAP` of the datapath.

The two maps are keyed differently on purpose. A floating IP is unique across the
whole setup, so the inbound direction is keyed by it alone and carries the tenant
of the VM in its value - resolving a floating IP is exactly the step that moves a
packet from the shared uplink into a tenant. The outbound direction is keyed by
`(vni, internal_ip)`, because the internal address is precisely the thing that may
repeat across tenants."###,
    error_code = 409,
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
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    let uuid = Uuid::new_v4();
    let mut state = GATEWAY_STATE_HANDLE.lock().await;

    // A floating IP names one VM of one tenant. Handing the same one to a second
    // tenant would make the inbound direction ambiguous, so it is refused.
    if let Some(existing) = state.floating_ips.get(&body.floating_ip)
        && (existing.vni != body.vni || existing.internal_ip != body.internal_ip)
    {
        return Err(ErrorResponse::Conflict(format!(
            "{} already points at {} in tenant {}",
            body.floating_ip, existing.internal_ip, existing.vni
        )));
    }

    state.floating_ips.insert(
        body.floating_ip,
        FloatingIp {
            vni: body.vni,
            internal_ip: body.internal_ip,
        },
    );

    let target = FipTarget {
        vni: body.vni,
        ip: u32::from(body.internal_ip),
    };
    if state
        .fip_dnat_map
        .insert(u32::from(body.floating_ip), FipTargetPod(target), 0)
        .is_err()
    {
        log::error!("eBPF Map error (DNAT)");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    let snat_key = RouteKeyPod(RouteKey::new(body.vni, u32::from(body.internal_ip)));
    if state
        .fip_snat_map
        .insert(snat_key, u32::from(body.floating_ip), 0)
        .is_err()
    {
        log::error!("eBPF Map error (SNAT)");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    let resp = FloatingIpInternalResp {
        uuid,
        name: body.name.clone(),
        network_uuid: body.network_uuid,
        floating_ip: body.floating_ip,
        internal_ip: body.internal_ip,
        vni: body.vni,
    };

    Ok(CreatedJson(resp))
}
