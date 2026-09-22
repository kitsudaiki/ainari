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

use actix_web::web::{Json, Path};
use apistos::api_operation;
use torii_common::RouteKey;
use uuid::Uuid;
use validator::Validate;

use crate::core::filter::apply_filter;
use crate::core::models::Route;
use crate::core::models::{RouteKeyPod, RouteTargetPod};
use crate::core::routing::{build_route_target, check_route_tenant};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::core::utils::{get_ifindex, validate_vni};

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "route",
    summary = "Update route",
    description = r###"Update an existing route atomically for zero-downtime migration.

Because eBPF map updates are atomic at the kernel level, active connections pivot
to the new tunnel destination without dropping packets. The floating-ip NAT maps
remain unaffected."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn update_route_internal(
    route_uuid: Path<Uuid>,
    body: Json<RouteReq>,
    _context: UserContext,
) -> Result<Json<RouteResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    let route_uuid = route_uuid.into_inner();

    // the destination address together with the tenant is the key of the eBPF route map
    let route_key = RouteKey::new(body.vni, u32::from(body.dest_ip));

    // resolve the new target interface and its link layer details
    if get_ifindex(&body.target_iface) == 0 {
        return Err(ErrorResponse::NotFound(format!(
            "Interface {} not found",
            body.target_iface
        )));
    }

    // Snapshot the TAP registry so the (possibly slow) ARP resolution inside
    // the target construction does not block the rest of the gateway.
    let taps = { GATEWAY_STATE_HANDLE.lock().await.taps.clone() };

    {
        let st = GATEWAY_STATE_HANDLE.lock().await;
        check_route_tenant(&st, &body, Some(route_uuid)).map_err(ErrorResponse::Conflict)?;
    }

    let target = build_route_target(&body, &taps).map_err(ErrorResponse::BadRequest)?;

    let mut st = GATEWAY_STATE_HANDLE.lock().await;

    let previous_key = match st.routes.get(&route_uuid) {
        Some(route) => RouteKey::new(route.vni, u32::from(route.dest_ip)),
        None => return Err(ErrorResponse::NotFound("Route UUID not found".to_string())),
    };

    let updated_route = Route {
        uuid: route_uuid,
        vni: body.vni,
        dest_ip: body.dest_ip,
        target_iface: body.target_iface.clone(),
        gateway_ip: body.gateway_ip,
        next_hop_ip: body.next_hop_ip,
        next_hop_mac: body.next_hop_mac.clone(),
        encrypted: body.encrypted,
    };

    // ATOMIC KERNEL UPDATE: overwriting the key redirects the traffic instantly,
    // without a delete/create gap.
    if st
        .route_map
        .insert(RouteKeyPod(route_key), RouteTargetPod(target), 0)
        .is_err()
    {
        log::error!("eBPF Map error on update");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    st.routes.insert(route_uuid, updated_route.clone());

    // A route that changed its destination *or its tenant* has to take its packet
    // filter with it, otherwise the new key would be reachable unfiltered while
    // the old one keeps an orphaned entry behind. The stale routing entry goes
    // away for exactly the same reason.
    if previous_key != route_key {
        let _ = st.route_map.remove(&RouteKeyPod(previous_key));
        let _ = st.filter_map.remove(&RouteKeyPod(previous_key));
        let rules = st.filters.get(&route_uuid).cloned().unwrap_or_default();
        apply_filter(&mut st, route_uuid, route_key, rules)
            .map_err(|e| map_internal_error("move packet-filter of route", e))?;
    }

    let updated_route = RouteResp {
        uuid: route_uuid,
        vni: body.vni,
        dest_ip: body.dest_ip,
        target_iface: body.target_iface.clone(),
        gateway_ip: body.gateway_ip,
        next_hop_ip: body.next_hop_ip,
        next_hop_mac: body.next_hop_mac.clone(),
        encrypted: body.encrypted,
    };

    Ok(Json(updated_route))
}
