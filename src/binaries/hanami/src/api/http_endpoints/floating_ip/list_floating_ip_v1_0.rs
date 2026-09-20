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

use crate::database::floating_ip_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "floating_ip",
    summary = "List floating_ips",
    description = r###"List basic information of all floating_ips from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_floating_ip(
    context: UserContext,
) -> Result<Json<FloatingIpListResp>, ErrorResponse> {
    // get floating_ips from db
    let floating_ips = floating_ip_table::list_floating_ips(&context)
        .map_err(|e| map_db_list_error("hosts", e))?;

    // prepare response
    let mut resp = FloatingIpListResp {
        floating_ips: Vec::new(),
    };

    // fill reponse
    for floating_ip in floating_ips {
        // add single object to the reponse-list
        let obj = FloatingIpBasicResp {
            uuid: floating_ip.uuid,
            network_uuid: floating_ip.network_uuid,
            floating_ip: floating_ip.floating_ip_addr,
            internal_ip: floating_ip.internal_ip_addr,
        };

        resp.floating_ips.push(obj);
    }

    Ok(Json(resp))
}
