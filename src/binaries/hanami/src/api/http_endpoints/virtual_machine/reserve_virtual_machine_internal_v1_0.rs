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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use super::reserve_virtual_machine_v1_0::{ReserveRequest, reserve};

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;

#[api_operation(
    tag = "virtual_machine",
    summary = "Reserve new virtual_machine (internal)",
    description = r###"Reserve a new virtual_machine for another component, for example to restore it
from a backup.

It works like the reservation of the users, but the number of cores and the memory in MiB are given
directly instead of a vm-type. With `uuid`, `internal_ip` and `mac_address` the virtual_machine
keeps the original values of the backup. A UUID, which is used by an active virtual_machine or by
another project, or an address, which is already used, is rejected with `409`. The UUID of a
deleted virtual_machine of the same project can be used again. The values, which are not given,
are selected like for a new virtual_machine.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn reserve_virtual_machine_internal(
    body: Json<VirtualMachineReserveInternalReq>,
    context: UserContext,
) -> Result<CreatedJson<VirtualMachineResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let request = ReserveRequest {
        uuid: body.uuid,
        name: body.name.clone(),
        network_uuid: body.network_uuid,
        number_of_cores: body.number_of_cores,
        memory_size: body.memory_size,
        disk_size: body.disk_size,
        isolated_host: body.isolated_host,
        host_uuid: None,
        internal_ip: body.internal_ip,
        mac_address: body.mac_address.clone(),
    };
    Ok(CreatedJson(reserve(&request, &context).await?))
}
