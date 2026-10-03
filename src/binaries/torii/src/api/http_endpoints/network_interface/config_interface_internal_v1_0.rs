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

use crate::core::interface::configure_interface;
use crate::core::utils::validate_vni;
use crate::database::network_interface_table;

use ainari_api::common_functions::map_db_write_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::user_context::UserContext;

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
    context: UserContext,
) -> Result<Json<IfaceConfigResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    // the configuration is persisted, so it is restored after a restart of the gateway. If that
    // fails, the interface is reverted to its previous configuration.
    configure_interface(&body, || {
        network_interface_table::set_network_interface(&body, &context)
            .map(|_| ())
            .map_err(|e| map_db_write_error(&format!("persist interface '{}'", body.iface_name), e))
    })
    .await?;

    let resp = IfaceConfigResp {
        iface_name: body.iface_name.clone(),
        ip_cidr: body.ip_cidr.clone(),
        up: body.up,
        vni: body.vni,
        fip_port: body.fip_port,
    };

    Ok(Json(resp))
}
