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

use crate::core::filter::route_filter_key;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Get route filter",
    description = r###"Show the packet filter currently attached to one route."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_filter_internal(
    route_uuid: Path<Uuid>,
    _context: UserContext,
) -> Result<Json<FilterResp>, ErrorResponse> {
    let route_uuid = route_uuid.into_inner();
    let st = GATEWAY_STATE_HANDLE.lock().await;

    let (dest_ip, _) = match route_filter_key(&st, &route_uuid) {
        Some(key) => key,
        None => return Err(ErrorResponse::NotFound("Route UUID not found".to_string())),
    };

    let rules = st.filters.get(&route_uuid).cloned().unwrap_or_default();
    let message = if rules.is_empty() {
        format!("{} is unfiltered", dest_ip)
    } else {
        format!(
            "{} allows {} IP range(s) and {} port(s)",
            dest_ip,
            rules.ip_ranges.len(),
            rules.ports.len()
        )
    };
    log::debug!("{}", message);

    let resp = FilterResp {
        route_uuid,
        dest_ip,
        filter: rules,
    };

    Ok(Json(resp))
}
