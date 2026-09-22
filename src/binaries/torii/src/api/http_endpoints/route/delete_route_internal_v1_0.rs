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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use torii_common::RouteKey;
use uuid::Uuid;

use crate::config::CONFIG;
use crate::core::crypto::remove_block_policies;
use crate::core::models::RouteKeyPod;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::{run_ip, with_table};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "route",
    summary = "Delete route",
    description = r###"Delete a route and purge it from the eBPF maps.

The packet filter guarding the route dies with it, and an encrypted route also
loses its fail-closed block policies and its kernel host-route."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_route_internal(
    route_uuid: Path<Uuid>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let route_uuid = route_uuid.into_inner();
    let mut st = GATEWAY_STATE_HANDLE.lock().await;

    let route = match st.routes.remove(&route_uuid) {
        Some(route) => route,
        None => return Err(ErrorResponse::NotFound("Route not found".to_string())),
    };

    let dest_key = RouteKeyPod(RouteKey::new(route.vni, u32::from(route.dest_ip)));
    let _ = st.route_map.remove(&dest_key);
    // The filter guards the route, so it dies with it.
    let _ = st.filter_map.remove(&dest_key);
    st.filters.remove(&route_uuid);

    if route.encrypted {
        // Drop the fail-closed policies together with the route they guard.
        remove_block_policies(route.dest_ip);
        let dest = format!("{}/32", route.dest_ip);
        let table = CONFIG.network.tenant_table(route.vni);
        let mut args = vec!["route", "del", dest.as_str()];
        with_table(&mut args, &table);
        let _ = run_ip(&args);
    }

    Ok(NoContent)
}
