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

use crate::core::filter::{apply_filter, route_filter_key};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Clear route filter",
    description = r###"Drop the whole packet filter of one route.

Both include-lists are emptied in one step, which puts the route back into its
unrestricted default state."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn clear_filter_internal(
    route_uuid: Path<Uuid>,
    _context: UserContext,
) -> Result<Json<FilterResp>, ErrorResponse> {
    let route_uuid = route_uuid.into_inner();
    let mut st = GATEWAY_STATE_HANDLE.lock().await;

    let (dest_ip, dest_key) = match route_filter_key(&st, &route_uuid) {
        Some(key) => key,
        None => return Err(ErrorResponse::NotFound("Route UUID not found".to_string())),
    };

    apply_filter(&mut st, route_uuid, dest_key, RouteFilterRules::default())
        .map_err(|e| map_internal_error("clear packet-filter", e))?;

    let message = format!(
        "Packet filter of {} cleared, every address and port allowed",
        dest_ip
    );
    log::debug!("{}", message);

    let resp = FilterResp {
        route_uuid,
        dest_ip,
        filter: st.filters.get(&route_uuid).cloned().unwrap_or_default(),
    };

    Ok(Json(resp))
}
