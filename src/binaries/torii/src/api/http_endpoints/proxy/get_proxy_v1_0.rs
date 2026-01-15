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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::proxy_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;

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
    let proxy_data = proxy_table::get_proxy(&proxy_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("proxy", &proxy_uuid, e))?;

    let model_uuid = convert_uuid(&proxy_data.model_uuid)?;

    let resp = ProxyResp {
        uuid: *proxy_uuid,
        port: proxy_data.port as u16,
        target_address: proxy_data.target_address,
        model_uuid,
        created_by: proxy_data.created_by,
        created_at: proxy_data.created_at,
        updated_by: proxy_data.updated_by,
        updated_at: proxy_data.updated_at,
    };

    Ok(Json(resp))
}
