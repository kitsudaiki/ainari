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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::core::floating_ip::{detach_floating_ip as detach, to_floating_ip_resp};
use crate::database::floating_ip_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "floating_ip",
    summary = "Detach floating_ip",
    description = r###"Detach a floating_ip from its virtual_machine.

The NAT of the floating ip-address is removed from the torii, but the floating ip-address stays
reserved, so it can be attached to another virtual_machine again. Detaching a floating_ip,
which is not attached, has no effect."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn detach_floating_ip(
    floating_ip_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<FloatingIpResp>, ErrorResponse> {
    // get the floating_ip from the database to know, which address has to be removed from the NAT
    // of the torii. It also checks, that the user is allowed to detach the floating_ip.
    let floating_ip_entry = floating_ip_table::get_floating_ip(&floating_ip_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("floating_ip", &floating_ip_uuid, e))?;

    detach(&floating_ip_entry, &context).await?;

    let floating_ip_entry = floating_ip_table::get_floating_ip(&floating_ip_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("floating_ip", &floating_ip_uuid, e))?;

    Ok(Json(to_floating_ip_resp(floating_ip_entry)))
}
