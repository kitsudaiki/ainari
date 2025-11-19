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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::core::proxy_handler::*;
use crate::database::proxy_table;

use ainari_api::common_functions::map_ainari_error_to_api_response;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "proxy",
    summary = "Register new proxy",
    description = r###"Register new proxy."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn register_proxy_internal(
    body: Json<ProxyCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ProxyResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    let proxy_uuid = Uuid::new_v4();

    // add new proxy to database
    match proxy_table::add_new_proxy(
        &proxy_uuid,
        body.port,
        &body.target_address,
        &body.cluster_uuid,
        &context,
    ) {
        Ok(_) => {}
        Err(_) => {
            let msg = format!("Failed to add proxy with UUID '{proxy_uuid}' to database.");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // create new proxy and add it to the handler
    let mut proxy_handler = PROXY_HANDLER.write().await;
    proxy_handler
        .add_proxy(&proxy_uuid, body.port, &body.target_address)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // get new created proxy from database to get addtional information
    let proxy_data: proxy_table::ProxyEntry = match proxy_table::get_proxy(&proxy_uuid, &context) {
        Ok(proxy_data) => proxy_data,
        Err(_) => {
            let msg = format!(
                "Failed to get proxy with ID '{proxy_uuid}' from database, even the proxy should exist."
            );
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let resp = ProxyResp {
        uuid: proxy_uuid,
        port: body.port,
        target_address: proxy_data.target_address.clone(),
        cluster_uuid: body.cluster_uuid,
        created_by: proxy_data.created_by.clone(),
        created_at: proxy_data.created_at.clone(),
        updated_by: proxy_data.updated_by.clone(),
        updated_at: proxy_data.updated_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
