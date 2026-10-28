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

use crate::core::proxy_handler::proxy_conf;
use crate::database::proxy_table;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "proxy",
    summary = "List proxies",
    description = r###"List basic information of all proxies from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_proxy(context: UserContext) -> Result<Json<ProxyListResp>, ErrorResponse> {
    let proxys = proxy_table::list_proxys(&context)
        .map_err(|e| map_db_list_error("proxys", e, Some(&context)))?;

    let mut resp = ProxyListResp { proxys: Vec::new() };
    // a gateway without proxies has no config of them, but still answers with an empty list
    if proxys.is_empty() {
        return Ok(Json(resp));
    }
    let proxy_conf =
        proxy_conf().map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    for proxy in proxys {
        let uuid = proxy.uuid;
        let virtual_machine_uuid = proxy.virtual_machine_uuid;
        let obj = ProxyBasicResp {
            uuid,
            address: proxy_conf.address(&uuid),
            target_address: proxy.target_address,
            virtual_machine_uuid,
        };

        resp.proxys.push(obj);
    }

    Ok(Json(resp))
}
