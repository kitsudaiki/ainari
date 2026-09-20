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

use crate::database::virtual_machine_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;

#[api_operation(
    tag = "virtual_machine",
    summary = "List virtual_machines",
    description = r###"List basic information of all virtual_machines from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_virtual_machine_internal(
    context: UserContext,
) -> Result<Json<VirtualMachineListResp>, ErrorResponse> {
    let virtual_machines = match virtual_machine_table::list_virtual_machines(&context) {
        Ok(virtual_machines) => virtual_machines,
        Err(e) => {
            log::error!("Failed to get list of virtual_machines form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let mut resp = VirtualMachineListResp {
        virtual_machines: Vec::new(),
    };

    for virtual_machine in virtual_machines {
        let obj = VirtualMachineBasicResp {
            uuid: virtual_machine.uuid,
            name: virtual_machine.name,
            proxy_port: 0,
        };

        resp.virtual_machines.push(obj);
    }

    Ok(Json(resp))
}
