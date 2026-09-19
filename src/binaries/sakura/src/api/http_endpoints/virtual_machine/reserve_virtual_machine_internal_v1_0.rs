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
use uuid::Uuid;
use validator::Validate;

use crate::database::virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;

#[api_operation(
    tag = "virtual_machine",
    summary = "Create new virtual_machine",
    description = r###"Create new virtual_machine."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_virtual_machine_internal(
    body: Json<VirtualMachineInternalCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<VirtualMachineResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let virtual_machine_uuid = Uuid::new_v4();

    // these values coming from the task, so they are temporary filled with nil-values
    let empty_image_uuid = Uuid::nil();
    let empty_public_key_uuid = Uuid::nil();

    let new_virtual_machine = virtual_machine_table::NewVirtualMachine {
        uuid: virtual_machine_uuid,
        name: body.name.clone(),
        number_of_cores: body.number_of_cores,
        memory_size: body.memory_size,
        image_uuid: empty_image_uuid,
        public_key_uuid: empty_public_key_uuid,
        network_uuid: body.network_uuid,
        internal_ip: body.internal_ip,
        root_disk_path: None,
        seed_path: "".to_owned(),
        tap_name: body.tap_name.clone(),
        mac_address: body.mac_address.clone(),
    };

    virtual_machine_table::add_new_virtual_machine(new_virtual_machine, &context).map_err(|e| {
        log::error!(
            "Failed to add virtual-machine with UUID '{virtual_machine_uuid}' to database.: {e}"
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let virtual_machine_data =
        virtual_machine_table::get_virtual_machine(&virtual_machine_uuid, &context).map_err(
            |e| map_db_uuid_get_delete_error("virtual_machine", &virtual_machine_uuid, e),
        )?;

    let resp = VirtualMachineResp {
        uuid: virtual_machine_uuid,
        name: virtual_machine_data.name,
        is_created: virtual_machine_data.is_created,
        number_of_cores: virtual_machine_data.number_of_cores,
        memory_size: virtual_machine_data.memory_size,
        image_uuid: virtual_machine_data.image_uuid,
        network_uuid: virtual_machine_data.network_uuid,
        internal_ip: virtual_machine_data.internal_ip,
        torii_port: 0,
        created_by: virtual_machine_data.created_by,
        created_at: virtual_machine_data.created_at,
        updated_by: virtual_machine_data.updated_by,
        updated_at: virtual_machine_data.updated_at,
    };

    Ok(CreatedJson(resp))
}
