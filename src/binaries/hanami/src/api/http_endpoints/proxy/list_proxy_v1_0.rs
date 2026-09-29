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

use crate::config;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::proxy_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::*;
use ainari_clients::proxy as proxy_clients;

#[api_operation(
    tag = "proxy",
    summary = "List proxies",
    description = r###"List basic information of all proxies, which make the sakura-hosts reachable over the ports of the torii at the edge."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_proxy(context: UserContext) -> Result<Json<ProxyListResp>, ErrorResponse> {
    // the proxies are stored in the torii at the edge, whose api is only reachable within the
    // cluster, so hanami requests them there with the token of the user
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let proxy_list = proxy_clients::list_proxy(
        &endpoints.torii,
        &context.token,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok(Json(proxy_list))
}
