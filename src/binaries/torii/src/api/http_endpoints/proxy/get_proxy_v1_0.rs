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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::proxy_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "proxy",
    summary = "Get proxy",
    description = r###"Get information of a proxy from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_proxy(
    proxy_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<ProxyResp>, ErrorResponse> {
    let proxy_data = match proxy_table::get_proxy(&proxy_uuid, &context) {
        Ok(proxy_data) => proxy_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Proxy with UUID '{proxy_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let cluster_uuid = match Uuid::parse_str(&proxy_data.cluster_uuid) {
        Ok(cluster_uuid) => cluster_uuid,
        Err(e) => {
            log::error!("Failed to convert cluster-uuid with error: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    let resp = ProxyResp {
        uuid: *proxy_uuid,
        port: proxy_data.port as u16,
        target_address: proxy_data.target_address.clone(),
        cluster_uuid,
        created_by: proxy_data.created_by.clone(),
        created_at: proxy_data.created_at.clone(),
        updated_by: proxy_data.updated_by.clone(),
        updated_at: proxy_data.updated_at.clone(),
    };

    return Ok(Json(resp));
}
