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
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;

#[api_operation(
    tag = "virtual_machine",
    summary = "Get virtual_machine",
    description = r###"Get information of a virtual_machine from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_virtual_machine_internal(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<VirtualMachineResp>, ErrorResponse> {
    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context).map_err(
            |e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e),
        )?;

    let resp = VirtualMachineResp {
        uuid: *virtual_machine_uuid,
        is_created: virtual_machine_data.is_created,
        number_of_cores: virtual_machine_data.number_of_cores,
        memory_size: virtual_machine_data.memory_size,
        disk_size: virtual_machine_data.disk_size,
        image_uuid: virtual_machine_data.image_uuid,
        name: virtual_machine_data.name,
        network_uuid: virtual_machine_data.network_uuid,
        internal_ip: virtual_machine_data.internal_ip,
        torii_port: 0,
        created_by: virtual_machine_data.created_by,
        created_at: virtual_machine_data.created_at,
        updated_by: virtual_machine_data.updated_by,
        updated_at: virtual_machine_data.updated_at,
    };

    Ok(Json(resp))
}
