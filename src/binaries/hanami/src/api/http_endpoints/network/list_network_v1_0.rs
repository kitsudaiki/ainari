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

use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network",
    summary = "List network",
    description = r###"List basic information of all network from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_network(context: UserContext) -> Result<Json<NetworkListResp>, ErrorResponse> {
    // get networks from db
    let networks =
        network_table::list_networks(&context).map_err(|e| map_db_list_error("hosts", e))?;

    // prepare response
    let mut resp = NetworkListResp {
        networks: Vec::new(),
    };

    // fill reponse
    for network in networks {
        // add single object to the reponse-list
        let obj = NetworkBasicResp {
            uuid: network.uuid,
            name: network.name,
            subnet: network.subnet,
        };

        resp.networks.push(obj);
    }

    Ok(Json(resp))
}
