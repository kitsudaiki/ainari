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

use crate::config;
use crate::database::host_table;
use crate::database::meta_cluster_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::cluster_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::cluster as cluster_clients;
use ainari_clients::endpoints::*;
use ainari_clients::proxy as proxy_clients;

#[api_operation(
    tag = "cluster",
    summary = "Get cluster",
    description = r###"Get information of a cluster from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_cluster(
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<ClusterResp>, ErrorResponse> {
    let cluster_data = meta_cluster_table::get_meta_cluster(&cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("cluster-meta", &cluster_uuid, e))?;

    let sakura_uuid = convert_uuid(&cluster_data.sakura_host_uuid)?;

    let host_data = host_table::get_host(&sakura_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("sakura-host", &sakura_uuid, e))?;

    // get endpoints from miko
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // send request to torii to get port
    let proxy_uuid = convert_uuid(&cluster_data.proxy_uuid)?;
    let proxy_resp = proxy_clients::get_proxy(
        &endpoints.torii,
        &context.token,
        &proxy_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // get cluster-information from sakura-host
    let mut cluster_resp = cluster_clients::get_cluster(
        &host_data.address,
        &context.token,
        &config::INTERNAL_API_KEY,
        &cluster_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // set port in response
    cluster_resp.torii_port = proxy_resp.port;

    Ok(Json(cluster_resp))
}
