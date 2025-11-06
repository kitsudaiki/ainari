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
use crate::database::host_table;
use crate::database::meta_cluster_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::cluster_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::cluster as cluster_clients;
use ainari_clients::endpoints::*;
use ainari_clients::proxy as proxy_clients;
use ainari_clients::quota::get_quota;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "cluster",
    summary = "Create new cluster",
    description = r###"Create new cluster based on a cluster-template."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn create_cluster(
    body: Json<ClusterCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<ClusterResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    check_quota(&context).await?;

    // list all avaialble hosts
    let hosts = match host_table::list_hosts(&context) {
        Ok(hosts) => hosts,
        Err(e) => {
            log::error!("Failed to get list of hosts form database: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check that there is at least one host
    if hosts.is_empty() {
        log::error!("No hosts to schedule new cluster.");
        return Err(ErrorResponse::InternalError("".to_string()));
    }

    // select first host
    // TODO: also be able to select one of many hosts
    let selected_host = if let Some(host) = hosts.first() {
        host
    } else {
        log::error!("No hosts with list-position 0 doesn't exist.");
        return Err(ErrorResponse::InternalError("".to_string()));
    };

    // send request to the selected sakura-host to create a cluster
    let mut cluster_resp = match cluster_clients::create_cluster(
        &selected_host.address,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &body.name,
        &body.template,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get endpoints from miko
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = match get_endpoints(miko_endpoint, config::CONFIG.insecure_clients).await {
        Ok(body) => body,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // send request to torii to create a proxy
    let proxy_resp = match proxy_clients::create_proxy(
        &endpoints.torii,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &cluster_resp.uuid,
        &selected_host.address,
        10042,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // set port-number for the response
    cluster_resp.torii_port = proxy_resp.port;

    // parse uuid-string
    let sakura_uuid = match Uuid::parse_str(&selected_host.uuid) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!("Failed to convert cluster-uuid with error: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // add new cluster to database
    let cluster_uuid = cluster_resp.uuid;
    match meta_cluster_table::add_new_meta_cluster(
        &cluster_uuid,
        &sakura_uuid,
        &proxy_resp.uuid,
        &context,
    ) {
        Ok(_) => {}
        Err(_) => {
            let msg = format!("Failed to add cluster with UUID '{cluster_uuid}' to database.");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    return Ok(CreatedJson(cluster_resp));
}

async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of meta_clusters of the user
    let current_number_of_meta_clusters = match meta_cluster_table::count_meta_clusters(context) {
        Ok(number) => number,
        Err(e) => {
            log::error!("Failed to count meta_clusters in database.: {e}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check the maximum number of meta_clusters defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let max_number_of_meta_clusters = match get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body.max_cluster as i64,
        Err(AinariError::Unauthorized(msg)) => {
            return Err(ErrorResponse::Unauthorized(msg));
        }
        Err(AinariError::InvalidInput(msg)) => {
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check if quota is already exceeded
    if current_number_of_meta_clusters as i64 >= max_number_of_meta_clusters {
        return Err(ErrorResponse::Conflict(
            "Maximum number of meta_clusters exceeded.".to_string(),
        ));
    }

    Ok(())
}
