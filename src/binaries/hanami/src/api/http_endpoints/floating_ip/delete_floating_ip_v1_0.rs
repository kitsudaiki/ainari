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
use uuid::Uuid;

use crate::core::floating_ip::detach_floating_ip;
use crate::database::floating_ip_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "floating_ip",
    summary = "Delete floating_ip",
    description = r###"Delete a floating_ip.

If the floating_ip is still attached to a virtual_machine, it is detached first, so the
address is never released while the torii still translates it."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_floating_ip(
    floating_ip_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // get the floating_ip from the database to know, if it is still attached. It also checks,
    // that the user is allowed to delete the floating_ip.
    let floating_ip_entry = floating_ip_table::get_floating_ip(&floating_ip_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("floating_ip", &floating_ip_uuid, e))?;

    // remove the NAT of the floating ip-address from the torii before releasing the address, so
    // the address is never free again, while the torii still translates it
    detach_floating_ip(&floating_ip_entry, &context).await?;

    floating_ip_table::delete_floating_ip(&floating_ip_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("floating_ip", &floating_ip_uuid, e))?;

    Ok(NoContent)
}
