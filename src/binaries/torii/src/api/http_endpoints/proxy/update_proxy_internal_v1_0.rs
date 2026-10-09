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

use actix_web::web::{Json, Path};
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::core::proxy_handler::*;
use crate::database::proxy_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "proxy",
    summary = "Update proxy",
    description = r###"Point a proxy at another target address.

The proxy keeps its UUID and its port, so its virtual_machine stays reachable under the same
port, for example after it was migrated to another sakura-host. Open connections over the proxy
are closed."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn update_proxy_internal(
    proxy_uuid: Path<Uuid>,
    body: Json<ProxyUpdateReq>,
    context: UserContext,
) -> Result<Json<ProxyResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // the database is changed first, so the proxy-handler gets the same target after a restart
    let proxy_data = proxy_table::update_proxy_target(&proxy_uuid, &body.target_address, &context)
        .map_err(|e| map_db_uuid_get_delete_error("proxy", &proxy_uuid, e))?;

    let mut proxy_handler = PROXY_HANDLER.write().await;
    proxy_handler
        .retarget_proxy(&proxy_uuid, proxy_data.port as u16, &body.target_address)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let resp = ProxyResp {
        uuid: proxy_data.uuid,
        port: proxy_data.port as u16,
        target_address: proxy_data.target_address,
        virtual_machine_uuid: proxy_data.virtual_machine_uuid,
        created_by: proxy_data.created_by,
        created_at: proxy_data.created_at,
        updated_by: proxy_data.updated_by,
        updated_at: proxy_data.updated_at,
    };

    Ok(Json(resp))
}
