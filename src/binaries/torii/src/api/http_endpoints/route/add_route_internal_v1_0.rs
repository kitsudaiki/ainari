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

use crate::config::CONFIG;
use crate::core::routing::add_route;
use crate::core::utils::validate_vni;
use crate::database::route_table;

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "route",
    summary = "Register new route",
    description = r###"Register a new route and program it into the eBPF maps of the datapath.

The route is stored under `(vni, dest_ip)`, so the same destination may exist
once per tenant. A request without a `vni` lands in tenant 0, which is the shared
tenant every setup that does not care about isolation runs in."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn register_route_internal(
    body: Json<RouteReq>,
    context: UserContext,
) -> Result<CreatedJson<RouteResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    // a caller, which doesn't know the interfaces of this gateway, can leave the target-interface
    // empty to address the underlay of this gateway
    let mut body = body.into_inner();
    if body.target_iface.is_empty() {
        body.target_iface = CONFIG.network.underlay_iface.clone();
    }

    let route = add_route(Uuid::new_v4(), &body).await?;

    // persist the route, so it is restored after a restart of the gateway
    route_table::add_new_route(&route, &context)
        .map_err(|e| map_internal_error(&format!("persist route '{}'", route.uuid), e))?;

    let route = RouteResp {
        uuid: route.uuid,
        vni: body.vni,
        dest_ip: body.dest_ip,
        target_iface: body.target_iface.clone(),
        gateway_ip: body.gateway_ip,
        next_hop_ip: body.next_hop_ip,
        next_hop_mac: body.next_hop_mac.clone(),
        encrypted: body.encrypted,
    };

    Ok(CreatedJson(route))
}
