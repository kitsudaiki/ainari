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

use crate::config;
use crate::core::proxy_handler::*;
use crate::database::proxy_table;

use ainari_api::common_functions::*;
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
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let proxy_uuid = Uuid::new_v4();

    let free_port =
        proxy_table::get_free_proxy(config::CONFIG.ports.min_port, config::CONFIG.ports.max_port)?;

    // add new proxy to database
    proxy_table::add_new_proxy(
        &proxy_uuid,
        free_port,
        &body.target_address,
        &body.cluster_uuid,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add proxy with UUID '{proxy_uuid}' to database with error: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // create new proxy and add it to the handler
    let mut proxy_handler = PROXY_HANDLER.write().await;
    proxy_handler
        .add_proxy(&proxy_uuid, free_port, &body.target_address)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // get new created proxy from database to get addtional information
    let proxy_data = proxy_table::get_proxy(&proxy_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("proxy", &proxy_uuid, e))?;

    let resp = ProxyResp {
        uuid: proxy_uuid,
        port: proxy_data.port as u16,
        target_address: proxy_data.target_address,
        cluster_uuid: body.cluster_uuid,
        created_by: proxy_data.created_by,
        created_at: proxy_data.created_at,
        updated_by: proxy_data.updated_by,
        updated_at: proxy_data.updated_at,
    };

    Ok(CreatedJson(resp))
}
