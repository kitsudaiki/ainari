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

use crate::core::filter::{apply_filter, filter_resp, filter_slot, persist_filter};
use crate::core::models::FilterKey;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;
use crate::database::network_filter_table;

use ainari_api::common_functions::{map_db_write_error, map_internal_error};
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Clear filter",
    description = r###"Drop the whole packet filter of one direction of an address.

Both include-lists are emptied in one step, which puts the traffic of this
direction back into its unrestricted default state."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn clear_filter_internal(
    path: Path<FilterPath>,
    context: UserContext,
) -> Result<Json<FilterResp>, ErrorResponse> {
    let key = FilterKey::from(path.into_inner());
    let mut st = GATEWAY_STATE_HANDLE.lock().await;
    let slot = filter_slot(&st, &key).map_err(ErrorResponse::NotFound)?;

    let previous = st.filters.get(&key).cloned().unwrap_or_default();
    apply_filter(&mut st, key, slot, RouteFilterRules::default())
        .map_err(|e| map_internal_error("clear packet-filter", e))?;

    // persist the new include-lists, so they are restored after a restart of the gateway. If
    // that fails, the previous include-lists are applied again.
    persist_filter(&mut st, key, slot, previous, |rules| {
        network_filter_table::set_filter_rules(&key, rules, &context)
            .map_err(|e| map_db_write_error("persist packet-filter", e))
    })?;

    log::debug!(
        "{} packet filter of {} cleared, every address and port allowed",
        key.direction,
        key.ip
    );

    Ok(Json(filter_resp(&st, key)))
}
