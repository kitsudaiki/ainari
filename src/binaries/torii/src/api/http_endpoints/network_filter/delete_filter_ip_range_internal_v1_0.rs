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

use crate::core::filter::{apply_filter, route_filter_key};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::common_functions::map_internal_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Remove filter ip-ranges",
    description = r###"Remove IP ranges from the include-list of one route.

An entry is identified by the addresses it covers, not by the way it was written
down: `10.0.0.0/24` and `10.0.0.0-10.0.0.255` remove the same entry. Removing the
last range opens the route for every address again."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_filter_ip_range_internal(
    route_uuid: Path<Uuid>,
    body: Json<FilterIpRangeReq>,
    _context: UserContext,
) -> Result<Json<FilterResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let route_uuid = route_uuid.into_inner();

    if body.ranges.is_empty() {
        return Err(ErrorResponse::BadRequest("No IP range given".to_string()));
    }

    let mut st = GATEWAY_STATE_HANDLE.lock().await;
    let (vni, dest_ip, dest_key) = match route_filter_key(&st, &route_uuid) {
        Some(key) => key,
        None => return Err(ErrorResponse::NotFound("Route UUID not found".to_string())),
    };

    let mut rules = st.filters.get(&route_uuid).cloned().unwrap_or_default();
    let before = rules.ip_ranges.len();
    rules.ip_ranges.retain(|existing| {
        !body
            .ranges
            .iter()
            .any(|rule| rule.first == existing.first && rule.last == existing.last)
    });
    let removed = before - rules.ip_ranges.len();

    apply_filter(&mut st, route_uuid, dest_key, rules)
        .map_err(|e| map_internal_error("apply packet-filter", e))?;

    let remaining = st
        .filters
        .get(&route_uuid)
        .map_or(0, |rules| rules.ip_ranges.len());
    let message = if remaining == 0 {
        format!(
            "{} IP range(s) removed, {} accepts every address again",
            removed, dest_ip
        )
    } else {
        format!(
            "{} IP range(s) removed, {} left in the include-list of {}",
            removed, remaining, dest_ip
        )
    };
    log::debug!("{}", message);

    let rules = st.filters.get(&route_uuid).cloned().unwrap_or_default();
    let resp = FilterResp {
        route_uuid,
        vni,
        dest_ip,
        filter: rules,
    };

    Ok(Json(resp))
}
