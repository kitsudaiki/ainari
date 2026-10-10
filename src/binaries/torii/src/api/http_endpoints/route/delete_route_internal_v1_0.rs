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
use uuid::Uuid;

use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::mls_key_exchange::refresh_network_keys;
use crate::core::routing::remove_route;
use crate::database::route_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "route",
    summary = "Delete route",
    description = r###"Delete a route and purge it from the eBPF maps.

The packet filters of its destination die with it, and an encrypted route also
loses its fail-closed block policies and its kernel host-route."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_route_internal(
    route_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let route_uuid = route_uuid.into_inner();
    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;

    let route = match ebpf_interf.routes.get(&route_uuid) {
        Some(route) => route.clone(),
        None => return Err(ErrorResponse::NotFound("Route not found".to_string())),
    };

    // The route and its packet-filters are dropped from the database first, so a failing database
    // leaves the route untouched in the datapath. The routes, which the gateway derives from its
    // own config at startup, have no entry there.
    if let Err(enums::DbError::InternalError) = route_table::delete_route(&route, &context) {
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    remove_route(&mut ebpf_interf, &route_uuid);
    drop(ebpf_interf);

    // the connections of the VM behind the route lose their keys
    refresh_network_keys(route.vni).await;

    Ok(NoContent)
}
