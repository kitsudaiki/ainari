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

use actix_web::web::Json;
use apistos::api_operation;

use crate::core::filter::filter_resp;
use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "List filters",
    description = r###"List every packet filter of the gateway.

Addresses without any include-list are left out: they carry everything and have
no entry in the eBPF filter maps either."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_filter_internal(
    _context: UserContext,
) -> Result<Json<FilterListResponse>, ErrorResponse> {
    let st = GATEWAY_STATE_HANDLE.lock().await;

    let mut filters: Vec<FilterResp> = st
        .filters
        .keys()
        .map(|key| filter_resp(&st, *key))
        .collect();
    filters.sort_by_key(|entry| (entry.vni, entry.ip, entry.direction));

    Ok(Json(FilterListResponse { filters }))
}
