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
use validator::Validate;

use crate::core::proxy_handler::*;
use crate::database::proxy_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "proxy",
    summary = "Register new proxy",
    description = r###"Register a new proxy.

The proxy gets the UUID of its virtual_machine, so it is reachable under the address
`https://<virtual_machine-uuid>.<domain>:<port>` and every virtual_machine has at most one proxy.
All proxies share one port and the proxy of a connection is picked by the server
name of its TLS ClientHello, so the TLS-connection reaches the target unchanged."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn register_proxy_internal(
    body: Json<ProxyCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ProxyResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let proxy_conf =
        proxy_conf().map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    // the proxy has the uuid of its virtual_machine, so the user sees in the address of the
    // proxy, which virtual_machine it reaches
    let proxy_uuid = body.virtual_machine_uuid;
    // the uuid of a deleted proxy is free again, for example for a virtual_machine, which is
    // restored with its uuid
    let uuid_in_use = proxy_table::is_proxy_uuid_in_use(&proxy_uuid)
        .map_err(|e| map_db_list_error("proxys", e, Some(&context)))?;
    if uuid_in_use {
        return Err(ErrorResponse::Conflict(format!(
            "The virtual_machine '{proxy_uuid}' already has a proxy."
        )));
    }

    // add new proxy to database
    proxy_table::add_new_proxy(
        &proxy_uuid,
        &body.target_address,
        &body.virtual_machine_uuid,
        &context,
    )
    .map_err(|e| {
        map_db_write_error(
            &format!("add proxy with UUID '{proxy_uuid}' to database"),
            e,
            Some(&context),
        )
    })?;

    // create new proxy and add it to the handler
    let mut proxy_handler = PROXY_HANDLER.write().await;
    proxy_handler
        .add_proxy(&proxy_uuid, &body.target_address)
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    // get new created proxy from database to get additional information
    let proxy_data = proxy_table::get_proxy(&proxy_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("proxy", &proxy_uuid, e, Some(&context)))?;

    let resp = ProxyResp {
        uuid: proxy_uuid,
        address: proxy_conf.address(&proxy_uuid),
        target_address: proxy_data.target_address,
        virtual_machine_uuid: body.virtual_machine_uuid,
        created_by: proxy_data.created_by,
        created_at: proxy_data.created_at,
        updated_by: proxy_data.updated_by,
        updated_at: proxy_data.updated_at,
    };

    Ok(CreatedJson(resp))
}
