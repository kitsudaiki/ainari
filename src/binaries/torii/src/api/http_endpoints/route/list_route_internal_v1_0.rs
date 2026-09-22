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

use crate::core::models::Route;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "route",
    summary = "List routes",
    description = r###"List all dynamically configured routes currently managing packet flow via eBPF."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_route_internal(
    _context: UserContext,
) -> Result<Json<RouteListResp>, ErrorResponse> {
    let st = GATEWAY_STATE_HANDLE.lock().await;

    let mut routes: Vec<Route> = st.routes.values().cloned().collect();
    routes.sort_by_key(|route| (route.vni, route.dest_ip));

    let mut resp = RouteListResp::default();
    for route in routes {
        let converted_route = RouteResp {
            uuid: route.uuid,
            vni: route.vni,
            dest_ip: route.dest_ip,
            target_iface: route.target_iface,
            gateway_ip: route.gateway_ip,
            next_hop_ip: route.next_hop_ip,
            next_hop_mac: route.next_hop_mac,
            encrypted: route.encrypted,
        };

        resp.routes.push(converted_route);
    }

    Ok(Json(resp))
}
