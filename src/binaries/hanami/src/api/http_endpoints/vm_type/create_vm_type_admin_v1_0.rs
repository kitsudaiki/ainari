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

use crate::database::vm_type_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::vm_type_structs::*;

#[api_operation(
    tag = "vm_type",
    summary = "Create new vm-type",
    description = r###"Create a new vm-type, which defines the number of cores and the amount of
memory in MiB of a virtual-machine. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_vm_type_admin(
    body: Json<VmTypeCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<VmTypeResp>, ErrorResponse> {
    check_admin_context(&context)?;

    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let vm_type_uuid = Uuid::new_v4();

    // add new vm-type to database
    vm_type_table::add_new_vm_type(
        &vm_type_uuid,
        &body.name,
        body.number_of_cores,
        body.amount_of_memory,
        &context,
    )
    .map_err(|e| {
        map_db_write_error(
            &format!("add vm-type with UUID '{vm_type_uuid}' to database"),
            e,
        )
    })?;

    // get new created vm-type from database to get additional information
    let vm_type = vm_type_table::get_vm_type(&vm_type_uuid)
        .map_err(|e| map_db_uuid_get_after_add_error("vm-type", &vm_type_uuid, e))?;

    let resp = VmTypeResp {
        uuid: vm_type_uuid,
        name: vm_type.name,
        number_of_cores: vm_type.number_of_cores,
        amount_of_memory: vm_type.amount_of_memory,
        created_by: vm_type.created_by,
        created_at: vm_type.created_at,
        updated_by: vm_type.updated_by,
        updated_at: vm_type.updated_at,
    };

    Ok(CreatedJson(resp))
}
