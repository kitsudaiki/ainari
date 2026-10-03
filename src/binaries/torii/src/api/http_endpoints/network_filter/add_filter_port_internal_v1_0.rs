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
use validator::Validate;

use crate::core::filter::{apply_filter, filter_resp, filter_slot, persist_filter};
use crate::core::models::FilterKey;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::database::network_filter_table;

use ainari_api::common_functions::map_db_write_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Add filter ports",
    description = r###"Add ports to the include-list of one direction of an address.

As long as the list is empty every port is allowed. Once it holds an entry, only
TCP and UDP packets with a matching source *or* destination port are carried -
matching either side is what lets the answers of an allowed service back through
a stateless filter. Traffic without ports (ICMP and friends) is not affected by
this list; it is governed by the IP ranges alone."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn add_filter_port_internal(
    path: Path<FilterPath>,
    body: Json<FilterPortReq>,
    context: UserContext,
) -> Result<Json<FilterResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let key = FilterKey::from(path.into_inner());

    if body.ports.is_empty() {
        return Err(ErrorResponse::BadRequest("No port given".to_string()));
    }

    let mut st = GATEWAY_STATE_HANDLE.lock().await;
    let slot = filter_slot(&st, &key).map_err(ErrorResponse::NotFound)?;

    let previous = st.filters.get(&key).cloned().unwrap_or_default();
    let mut rules = previous.clone();
    let mut added = 0;
    for rule in body.ports.iter().cloned() {
        let known = rules
            .ports
            .iter()
            .any(|existing| existing.first == rule.first && existing.last == rule.last);
        if !known {
            rules.ports.push(rule);
            added += 1;
        }
    }

    apply_filter(&mut st, key, slot, rules).map_err(ErrorResponse::BadRequest)?;

    // persist the new include-lists, so they are restored after a restart of the gateway. If
    // that fails, the previous include-lists are applied again.
    persist_filter(&mut st, key, slot, previous, |rules| {
        network_filter_table::set_filter_rules(&key, rules, &context)
            .map_err(|e| map_db_write_error("persist packet-filter", e))
    })?;

    let resp = filter_resp(&st, key);
    log::debug!(
        "{} port(s) added, {} in the {} include-list of {}",
        added,
        resp.filter.ports.len(),
        key.direction,
        key.ip
    );

    Ok(Json(resp))
}
