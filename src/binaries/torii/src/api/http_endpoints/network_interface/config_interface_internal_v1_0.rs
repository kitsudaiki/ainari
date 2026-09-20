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

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_interface",
    summary = "Configure interface",
    description = r###"Configure an existing network interface's IP and up/down state.

Acts as a wrapper around the system `ip` commands to programmatically assign
CIDRs and enable interfaces on the host operating system."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn config_interface_internal(
    body: Json<IfaceConfigReq>,
    _context: UserContext,
) -> Result<Json<IfaceConfigResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

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

    let resp = IfaceConfigResp {
        iface_name: body.iface_name.clone(),
        ip_cidr: body.ip_cidr.clone(),
        up: body.up,
    };

    Ok(Json(resp))
}
