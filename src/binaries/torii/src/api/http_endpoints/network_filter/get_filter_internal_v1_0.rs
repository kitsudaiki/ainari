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

use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::filter::{filter_resp, filter_slot};
use crate::core::models::FilterKey;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Get filter",
    description = r###"Show the packet filter currently attached to one direction of an address.

An address without a filter is reported with two empty include-lists."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_filter_internal(
    path: Path<FilterPath>,
    _context: UserContext,
) -> Result<Json<FilterResp>, ErrorResponse> {
    let key = FilterKey::from(path.into_inner());
    let ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;

    filter_slot(&ebpf_interf, &key).map_err(ErrorResponse::NotFound)?;

    Ok(Json(filter_resp(&ebpf_interf, key)))
}
