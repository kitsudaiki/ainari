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

use crate::core::filter::{apply_filter, build_filter_response, route_filter_key};
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Remove filter ports",
    description = r###"Remove ports from the include-list of one route.

Entries are matched by the ports they cover, so `22` removes the single port
entry and `8000-8100` the range. Removing the last entry opens the route for
every port again."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_filter_port_internal(
    route_uuid: Path<Uuid>,
    body: Json<FilterPortRequest>,
    _context: UserContext,
) -> Result<Json<FilterResponse>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let route_uuid = route_uuid.into_inner();

    if body.ports.is_empty() {
        return Err(ErrorResponse::BadRequest("No port given".to_string()));
    }

    let mut st = GATEWAY_STATE_HANDLE.lock().await;
    let (dest_ip, dest_key) = match route_filter_key(&st, &route_uuid) {
        Some(key) => key,
        None => return Err(ErrorResponse::NotFound("Route UUID not found".to_string())),
    };

    let mut rules = st.filters.get(&route_uuid).cloned().unwrap_or_default();
    let before = rules.ports.len();
    rules.ports.retain(|existing| {
        !body
            .ports
            .iter()
            .any(|rule| rule.first == existing.first && rule.last == existing.last)
    });
    let removed = before - rules.ports.len();

    apply_filter(&mut st, route_uuid, dest_key, rules).map_err(ErrorResponse::InternalError)?;

    let remaining = st
        .filters
        .get(&route_uuid)
        .map_or(0, |rules| rules.ports.len());
    let message = if remaining == 0 {
        format!(
            "{} port(s) removed, {} accepts every port again",
            removed, dest_ip
        )
    } else {
        format!(
            "{} port(s) removed, {} left in the include-list of {}",
            removed, remaining, dest_ip
        )
    };
    println!("{}", message);

    Ok(Json(build_filter_response(
        &st, route_uuid, dest_ip, message,
    )))
}
