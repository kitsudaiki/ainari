// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
use uuid::Uuid;

use crate::database::proxy_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "proxy",
    summary = "List proxy",
    description = r###"List basic information of all proxy from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_proxy(context: UserContext) -> Result<Json<ProxyListResp>, ErrorResponse> {
    let proxys = match proxy_table::list_proxys(&context) {
        Ok(proxys) => proxys,
        Err(e) => {
            log::error!("Failed to get list of proxys form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let mut resp = ProxyListResp { proxys: Vec::new() };

    for proxy in proxys {
        // parse-uuid-string coming from the database
        let uuid = match Uuid::parse_str(&proxy.uuid) {
            Ok(uuid) => uuid,
            Err(e) => {
                log::error!("Failed to convert proxy-uuid with error: '{e}'");
                return Err(ErrorResponse::InternalError("Internal Error".to_string()));
            }
        };

        let cluster_uuid = match Uuid::parse_str(&proxy.cluster_uuid) {
            Ok(cluster_uuid) => cluster_uuid,
            Err(e) => {
                log::error!("Failed to convert cluster-uuid with error: '{e}'");
                return Err(ErrorResponse::InternalError("Internal Error".to_string()));
            }
        };

        let obj = ProxyBasicResp {
            uuid,
            port: proxy.port as u16,
            target_address: proxy.target_address.clone(),
            cluster_uuid,
        };

        resp.proxys.push(obj);
    }

    Ok(Json(resp))
}
