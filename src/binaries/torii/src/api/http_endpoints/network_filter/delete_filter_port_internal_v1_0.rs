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

use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::filter::{apply_filter, filter_resp, filter_slot, persist_filter};
use crate::core::models::FilterKey;
use crate::database::network_filter_table;

use ainari_api::common_functions::{map_db_write_error, map_internal_error};
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Remove filter ports",
    description = r###"Remove ports from the include-list of one direction of an address.

Entries are matched by the ports they cover, so `22` removes the single port
entry and `8000-8100` the range. Removing the last entry opens the traffic of
this direction for every port again."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_filter_port_internal(
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

    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    let slot = filter_slot(&ebpf_interf, &key).map_err(ErrorResponse::NotFound)?;

    let previous = ebpf_interf.filters.get(&key).cloned().unwrap_or_default();
    let mut rules = previous.clone();
    let before = rules.ports.len();
    rules.ports.retain(|existing| {
        !body
            .ports
            .iter()
            .any(|rule| rule.first == existing.first && rule.last == existing.last)
    });
    let removed = before - rules.ports.len();

    apply_filter(&mut ebpf_interf, key, slot, rules)
        .map_err(|e| map_internal_error("apply packet-filter", e))?;

    // persist the new include-lists, so they are restored after a restart of the gateway. If
    // that fails, the previous include-lists are applied again.
    persist_filter(&mut ebpf_interf, key, slot, previous, |rules| {
        network_filter_table::set_filter_rules(&key, rules, &context)
            .map_err(|e| map_db_write_error("persist packet-filter", e))
    })?;

    let resp = filter_resp(&ebpf_interf, key);
    let remaining = resp.filter.ports.len();
    if remaining == 0 {
        log::debug!(
            "{} port(s) removed, {} {} accepts every port again",
            removed,
            key.direction,
            key.ip
        );
    } else {
        log::debug!(
            "{} port(s) removed, {} left in the {} include-list of {}",
            removed,
            remaining,
            key.direction,
            key.ip
        );
    }

    Ok(Json(resp))
}
