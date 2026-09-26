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
use validator::Validate;

use crate::core::floating_ip::{attach_floating_ip as attach, to_floating_ip_resp};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "floating_ip",
    summary = "Attach floating_ip",
    description = r###"Attach a floating_ip to a virtual_machine.

The network and the internal ip-address of the virtual_machine are read from the database and
the NAT of the floating ip-address is registered in the torii. A floating_ip can only be
attached to one virtual_machine and a virtual_machine can only have one floating_ip."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn attach_floating_ip(
    floating_ip_uuid: Path<Uuid>,
    body: Json<FloatingIpAttachReq>,
    context: UserContext,
) -> Result<Json<FloatingIpResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let floating_ip_entry = attach(&floating_ip_uuid, &body.virtual_machine_uuid, &context).await?;

    Ok(Json(to_floating_ip_resp(floating_ip_entry)))
}
