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

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Add filter ip-ranges",
    description = r###"Add IP ranges to the include-list of one route.

The list starts out empty, which means "every address is allowed". The first
entry flips that around: from then on the route only carries packets whose
source address is named by one of its ranges. Entries may be written as a single
address, as a subnet in CIDR notation or as an explicit range, and adding one
that is already present is a no-op rather than an error."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn add_filter_ip_range_internal(
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
    let (dest_ip, dest_key) = match route_filter_key(&st, &route_uuid) {
        Some(key) => key,
        None => return Err(ErrorResponse::NotFound("Route UUID not found".to_string())),
    };

    let mut rules = st.filters.get(&route_uuid).cloned().unwrap_or_default();
    let mut added = 0;
    for rule in body.ranges.iter().cloned() {
        let known = rules
            .ip_ranges
            .iter()
            .any(|existing| existing.first == rule.first && existing.last == rule.last);
        if !known {
            rules.ip_ranges.push(rule);
            added += 1;
        }
    }

    apply_filter(&mut st, route_uuid, dest_key, rules).map_err(ErrorResponse::BadRequest)?;

    let message = format!(
        "{} IP range(s) added, {} in the include-list of {}",
        added,
        st.filters
            .get(&route_uuid)
            .map_or(0, |rules| rules.ip_ranges.len()),
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
