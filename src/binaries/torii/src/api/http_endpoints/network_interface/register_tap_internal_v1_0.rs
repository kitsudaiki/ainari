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

use crate::core::interface::register_tap;
use crate::core::utils::validate_vni;
use crate::database::tap_table;

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_interface",
    summary = "Register new tap-device",
    description = r###"Create a new TAP device and dynamically attach the eBPF overlay program.

The device stays intentionally unnumbered - no IP address and no subnet are
attached to it, and any leftover address is flushed. Everything the VM needs from
a gateway is provided by eBPF instead: the XDP ARP responder answers the requests
of the VM (using the MAC of the TAP device itself) and the routing maps forward
the traffic. Because no subnet is claimed on the host side, several VMs of the
same subnet can be attached to the same host.

The device is also registered as a port of its tenant. That registration is the
only source of the VNI for everything the VM sends, which is what lets two VMs on
this host carry the very same address: they differ by the port their frames arrive
on, and every lookup downstream is keyed by that tenant.

In addition to the eBPF state, a host route and a permanent neighbour entry for
the VM are programmed into the kernel. They are what lets the kernel hand IPsec
protected traffic over to the right TAP after decrypting it, without ever needing
an address or an ARP exchange on this link. For a tenant other than the shared one
they go into that tenant's own routing table, selected by an `ip rule` on this TAP
- the kernel has no VNI, so the ingress interface is the only thing left to tell
the tenants apart on that path."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn register_tap_internal(
    body: Json<TapReq>,
    context: UserContext,
) -> Result<CreatedJson<TapResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    register_tap(&body).await?;

    // persist the registration, so the TAP is restored after a restart of the gateway
    tap_table::set_tap(&body, &context)
        .map_err(|e| map_internal_error(&format!("persist TAP '{}'", body.tap_name), e))?;

    let resp = TapResp {
        success: true,
        message: format!("TAP device configured successfully in tenant {}", body.vni),
        tap_name: body.tap_name.clone(),
        vni: body.vni,
    };

    Ok(CreatedJson(resp))
}
