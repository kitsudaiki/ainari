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

use crate::database::vm_type_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::vm_type_structs::*;

#[api_operation(
    tag = "vm_type",
    summary = "Update vm-type",
    description = r###"Update the values of a vm-type.

Only the values, which are set in the request, are applied, so single values can be changed
without providing all of them. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn update_vm_type_admin(
    vm_type_uuid: Path<Uuid>,
    body: Json<VmTypeUpdateReq>,
    context: UserContext,
) -> Result<Json<VmTypeResp>, ErrorResponse> {
    check_admin_context(&context)?;

    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // get current vm-type from database
    let current = vm_type_table::get_vm_type(&vm_type_uuid)
        .map_err(|e| map_db_uuid_get_delete_error("vm-type", &vm_type_uuid, e))?;

    // keep the current values, which are not set in the request
    let body = body.into_inner();
    let name = body.name.unwrap_or(current.name);
    let number_of_cores = body.number_of_cores.unwrap_or(current.number_of_cores);
    let amount_of_memory = body.amount_of_memory.unwrap_or(current.amount_of_memory);

    // update values in database
    vm_type_table::update_vm_type(
        &vm_type_uuid,
        &name,
        number_of_cores,
        amount_of_memory,
        &context,
    )
    .map_err(|e| map_db_uuid_get_delete_error("vm-type", &vm_type_uuid, e))?;

    // get updated vm-type from database
    let vm_type = vm_type_table::get_vm_type(&vm_type_uuid)
        .map_err(|e| map_db_uuid_get_delete_error("vm-type", &vm_type_uuid, e))?;

    let resp = VmTypeResp {
        uuid: vm_type.uuid,
        name: vm_type.name,
        number_of_cores: vm_type.number_of_cores,
        amount_of_memory: vm_type.amount_of_memory,
        created_by: vm_type.created_by,
        created_at: vm_type.created_at,
        updated_by: vm_type.updated_by,
        updated_at: vm_type.updated_at,
    };

    Ok(Json(resp))
}
