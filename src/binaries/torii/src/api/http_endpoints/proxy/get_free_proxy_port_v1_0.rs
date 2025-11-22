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

use crate::config;
use crate::database::proxy_table;

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
pub async fn get_free_proxy_port(_: UserContext) -> Result<Json<FreePortResp>, ErrorResponse> {
    let free_port =
        proxy_table::get_free_proxy(config::CONFIG.ports.min_port, config::CONFIG.ports.max_port)?;

    let resp = FreePortResp { port: free_port };

    Ok(Json(resp))
}
