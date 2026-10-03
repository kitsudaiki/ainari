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

use crate::core::floating_ip::add_floating_ip;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::validate_vni;
use crate::database::floating_ip_table;

use ainari_api::common_functions::map_db_write_error;
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
    context: UserContext,
) -> Result<CreatedJson<FloatingIpInternalResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    let uuid = Uuid::new_v4();

    {
        let mut state = GATEWAY_STATE_HANDLE.lock().await;
        // the floating ip is persisted, so it is restored after a restart of the gateway. If that
        // fails, it is removed from the datapath again.
        add_floating_ip(
            &mut state,
            body.floating_ip,
            body.vni,
            body.internal_ip,
            || {
                floating_ip_table::set_floating_ip(&uuid, &body, &context)
                    .map(|_| ())
                    .map_err(|e| {
                        map_db_write_error(
                            &format!("persist floating ip '{}'", body.floating_ip),
                            e,
                        )
                    })
            },
        )?;
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
