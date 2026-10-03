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
use apistos::api_operation;

use crate::database::vm_type_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::vm_type_structs::*;

#[api_operation(
    tag = "vm_type",
    summary = "List vm-types",
    description = r###"List basic information of all vm-types from the database.

The amount of memory is given in MiB."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_vm_type(_context: UserContext) -> Result<Json<VmTypeListResp>, ErrorResponse> {
    // get vm-types from db
    let vm_types = vm_type_table::list_vm_types().map_err(|e| map_db_list_error("vm-types", e))?;

    // prepare response
    let mut resp = VmTypeListResp {
        vm_types: Vec::new(),
    };

    // fill reponse
    for vm_type in vm_types {
        // add single object to the reponse-list
        let obj = VmTypeBasicResp {
            uuid: vm_type.uuid,
            name: vm_type.name,
            number_of_cores: vm_type.number_of_cores,
            amount_of_memory: vm_type.amount_of_memory,
        };

        resp.vm_types.push(obj);
    }

    Ok(Json(resp))
}
