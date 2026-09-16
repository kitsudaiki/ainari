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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;

use crate::core::routing_interface::GATEWAY_STATE_HANDLE;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::floating_ip_structs::*;

#[api_operation(
    tag = "floating_ip",
    summary = "Delete floating-ip",
    description = r###"Remove a floating-ip NAT configuration.

The NAT definitions are dropped from the internal tracking state as well as from
both eBPF NAT maps, which terminates the external access."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_floating_ip_internal(
    floating_ip: Path<FloatingIpPath>,
    _context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let floating_ip = floating_ip.into_inner().ip;

    let mut state = GATEWAY_STATE_HANDLE.lock().await;

    let internal_ip = match state.floating_ips.remove(&floating_ip) {
        Some(internal_ip) => internal_ip,
        None => return Err(ErrorResponse::NotFound("Floating IP not found".to_string())),
    };

    let _ = state.fip_dnat_map.remove(&u32::from(floating_ip));
    let _ = state.fip_snat_map.remove(&u32::from(internal_ip));

    Ok(NoContent)
}
