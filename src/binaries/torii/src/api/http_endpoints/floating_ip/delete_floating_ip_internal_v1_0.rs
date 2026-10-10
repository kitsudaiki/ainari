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

use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::floating_ip::remove_floating_ip;
use crate::database::floating_ip_table;

use ainari_api::common_functions::permission_denied_response;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

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
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let floating_ip = floating_ip.into_inner().ip;

    {
        let mut state = EBPF_INTERFACE_HANDLE.lock().await;
        if !state.floating_ips.contains_key(&floating_ip) {
            return Err(ErrorResponse::NotFound("Floating IP not found".to_string()));
        }

        // The floating ip is dropped from the database first, so a failing database leaves it
        // untouched in the datapath. A floating ip, which was registered before its persistence
        // was introduced, has no entry in the database, which is not an error here.
        match floating_ip_table::delete_floating_ip(&floating_ip, &context) {
            Ok(()) | Err(enums::DbError::NotFound) => {}
            Err(enums::DbError::InternalError) => {
                return Err(ErrorResponse::InternalError("Internal Error".to_string()));
            }
            Err(enums::DbError::PermissionDenied) => {
                return Err(permission_denied_response());
            }
        }

        remove_floating_ip(&mut state, floating_ip);
    }

    Ok(NoContent)
}
