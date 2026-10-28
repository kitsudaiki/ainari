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

use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::exports::SakuraVirtualMachineExport;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "virtual_machine",
    summary = "Export virtual_machine",
    description = r###"Export the config of a virtual_machine for a backup.

The config contains the resources, the image and public-key, which the virtual_machine was created
with, its network and addresses and its power-state. The root-disk is saved separately into the
backup. The seed is not exported, because it is generated again on restore.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn export_virtual_machine_internal(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<SakuraVirtualMachineExport>, ErrorResponse> {
    let virtual_machine = virtual_machine_table::get_virtual_machine(
        &virtual_machine_uuid,
        &context,
    )
    .map_err(|e| {
        map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e, Some(&context))
    })?;

    Ok(Json(SakuraVirtualMachineExport {
        uuid: virtual_machine.uuid,
        name: virtual_machine.name,
        vm_state: virtual_machine.vm_state,
        number_of_cores: virtual_machine.number_of_cores,
        memory_size: virtual_machine.memory_size,
        disk_size: virtual_machine.disk_size,
        image_uuid: virtual_machine.image_uuid,
        public_key_uuid: virtual_machine.public_key_uuid,
        network_uuid: virtual_machine.network_uuid,
        internal_ip: virtual_machine.internal_ip,
        mac_address: virtual_machine.mac_address,
    }))
}
