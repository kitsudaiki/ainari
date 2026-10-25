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

use crate::database::host_table;
use crate::database::meta_virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;

#[api_operation(
    tag = "virtual_machine",
    summary = "Get host of virtual_machine (internal)",
    description = r###"Get the sakura-host, which runs a virtual_machine.

Other components, like isekai for the backups, use the internal address of the host to call the
endpoints of sakura for the virtual_machine directly. The virtual_machine must belong to the
project of the user, unless the user is an admin.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_virtual_machine_host_internal(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<VirtualMachineHostResp>, ErrorResponse> {
    let virtual_machine_data =
        meta_virtual_machine_table::get_meta_virtual_machine(&virtual_machine_uuid, &context)
            .map_err(|e| {
                map_db_uuid_get_delete_error(
                    "virtual_machine",
                    &virtual_machine_uuid,
                    e,
                    Some(&context),
                )
            })?;

    let sakura_uuid = virtual_machine_data.sakura_host_uuid;
    let host_data = host_table::get_host(&sakura_uuid, &context).map_err(|e| {
        map_db_uuid_get_delete_error("sakura-host", &sakura_uuid, e, Some(&context))
    })?;

    Ok(Json(VirtualMachineHostResp {
        virtual_machine_uuid: *virtual_machine_uuid,
        host_uuid: host_data.uuid,
        address: host_data.address,
    }))
}
