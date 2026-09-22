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

use crate::core::models::IfaceConfigPod;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::{get_ifindex, validate_vni};

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::user_context::UserContext;
use torii_common::{IFACE_FLAG_FIP, IfaceConfig};

#[api_operation(
    tag = "network_interface",
    summary = "Configure interface",
    description = r###"Configure an existing network interface's IP and up/down state.

Acts as a wrapper around the system `ip` commands to programmatically assign
CIDRs and enable interfaces on the host operating system.

It is also where a port is placed into a tenant. Every packet entering the overlay
datapath takes its VNI from the interface it arrived on, so this registration is
what decides which half of the routing map a VM gets to see. `fip_port`
additionally marks the one interface that faces the outside world: only there is a
floating IP allowed to name the tenant of a packet, which is what keeps a VM from
reaching another tenant by addressing its floating IP."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn config_interface_internal(
    body: Json<IfaceConfigReq>,
    _context: UserContext,
) -> Result<Json<IfaceConfigResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    let name = &body.iface_name;

    if body.up {
        std::process::Command::new("ip")
            .args(["link", "set", name, "up"])
            .status()
            .map_err(|e| map_internal_error(&format!("bring interface '{name}' up"), e))?;
    }
    if let Some(ip) = &body.ip_cidr {
        let _ = std::process::Command::new("ip")
            .args(["addr", "add", ip, "dev", name])
            .status();
    }

    let ifindex = get_ifindex(name);
    if ifindex == 0 {
        return Err(ErrorResponse::NotFound(format!(
            "Interface {name} not found"
        )));
    }

    // Place the port into its tenant. An interface the control plane never
    // registered keeps behaving like a port of the shared tenant that translates
    // floating IPs, which is how the datapath worked before tenants existed.
    let flags = if body.fip_port { IFACE_FLAG_FIP } else { 0 };
    let cfg = IfaceConfig {
        vni: body.vni,
        flags,
    };
    {
        let mut st = GATEWAY_STATE_HANDLE.lock().await;
        if st
            .iface_map
            .insert(ifindex, IfaceConfigPod(cfg), 0)
            .is_err()
        {
            log::error!("eBPF Map error (interface)");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    }

    let resp = IfaceConfigResp {
        iface_name: body.iface_name.clone(),
        ip_cidr: body.ip_cidr.clone(),
        up: body.up,
        vni: body.vni,
        fip_port: body.fip_port,
    };

    Ok(Json(resp))
}
