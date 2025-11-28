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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::config;
use crate::database::host_table;
use crate::database::meta_cluster_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::cluster as cluster_clients;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::proxy as proxy_clients;

#[api_operation(
    tag = "cluster",
    summary = "Delete cluster",
    description = r###"Delete a cluster from the database and core."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_cluster(
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let cluster_data = meta_cluster_table::get_meta_cluster(&cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("cluster-meta", &cluster_uuid, e))?;

    let sakura_uuid = convert_uuid(&cluster_data.sakura_host_uuid)?;
    let proxy_uuid = convert_uuid(&cluster_data.proxy_uuid)?;

    let host_data = host_table::get_host(&sakura_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("sakura-host", &sakura_uuid, e))?;

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // send request to torii to delete the proxy, which is connected to the cluster
    proxy_clients::delete_proxy(
        &endpoints.torii,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &proxy_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // send request to sakura to delete the cluster
    cluster_clients::delete_cluster(
        &host_data.address,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &cluster_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // delete cluster from database of hanami
    meta_cluster_table::delete_meta_cluster(&cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("cluster-meta", &cluster_uuid, e))?;

    Ok(NoContent)
}
