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

use crate::database::host_table;

use ainari_api::common_functions::check_admin_context;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::host_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "host",
    summary = "List host",
    description = r###"List basic information of all host from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_host_admin(context: UserContext) -> Result<Json<HostListResp>, ErrorResponse> {
    check_admin_context(&context)?;

    let hosts = match host_table::list_hosts(&context) {
        Ok(hosts) => hosts,
        Err(e) => {
            log::error!("Failed to get list of hosts form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let mut resp = HostListResp { hosts: Vec::new() };

    for host in hosts {
        // parse-uuid-string coming from the database
        let uuid = match Uuid::parse_str(&host.uuid) {
            Ok(uuid) => uuid,
            Err(e) => {
                log::error!("Failed to convert host-uuid with error: '{e}'");
                return Err(ErrorResponse::InternalError("Internal Error".to_string()));
            }
        };

        let obj = HostBasicResp {
            uuid,
            name: host.name.clone(),
            host_address: host.address.clone(),
        };

        resp.hosts.push(obj);
    }

    Ok(Json(resp))
}
