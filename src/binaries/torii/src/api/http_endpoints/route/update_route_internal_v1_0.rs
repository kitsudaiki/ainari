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
use uuid::Uuid;
use validator::Validate;

use crate::core::routing::update_route;
use crate::core::utils::validate_vni;
use crate::database::route_table;

use ainari_api::common_functions::{map_db_write_error, permission_denied_response};
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

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
    context: UserContext,
) -> Result<Json<RouteResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    validate_vni(body.vni).map_err(ErrorResponse::BadRequest)?;

    let route_uuid = route_uuid.into_inner();

    // The update is persisted, so it survives a restart of the gateway. The routes, which the
    // gateway derives from its own config at startup, have no entry yet and get one with their
    // first update. If persisting fails, the previous version of the route is programmed again.
    update_route(route_uuid, &body, |route| {
        match route_table::update_route(route, &context) {
            Ok(()) => Ok(()),
            Err(enums::DbError::NotFound) => route_table::add_new_route(route, &context)
                .map(|_| ())
                .map_err(|e| map_db_write_error(&format!("persist route '{route_uuid}'"), e)),
            Err(enums::DbError::InternalError) => {
                Err(ErrorResponse::InternalError("Internal Error".to_string()))
            }
            Err(enums::DbError::PermissionDenied) => Err(permission_denied_response()),
        }
    })
    .await?;

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
