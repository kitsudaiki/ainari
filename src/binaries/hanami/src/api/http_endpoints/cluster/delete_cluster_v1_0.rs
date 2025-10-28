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

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::cluster as cluster_clients;
use ainari_clients::endpoints::*;
use ainari_clients::proxy as proxy_clients;
use ainari_common::enums;
use ainari_common::error::AinariError;

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
    let cluster_data = match meta_cluster_table::get_meta_cluster(&cluster_uuid, &context) {
        Ok(cluster_data) => cluster_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let sakura_uuid = match Uuid::parse_str(&cluster_data.sakura_host_uuid) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!("Failed to convert sakura-uuid with error: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let proxy_uuid = match Uuid::parse_str(&cluster_data.proxy_uuid) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!("Failed to convert proxy-uuid with error: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let host_data = match host_table::get_host(&sakura_uuid, &context) {
        Ok(host_data) => host_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Sakura-host with UUID '{sakura_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

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

    // send request to torii to delete the proxy, which is connected to the cluster
    match proxy_clients::delete_proxy(
        &endpoints.torii,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &proxy_uuid,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(()) => {}
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

    // send request to sakura to delete the cluster
    match cluster_clients::delete_cluster(
        &host_data.address,
        &context.token,
        &config::CONFIG.api.internal_api_key,
        &cluster_uuid,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(()) => {}
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

    // delete cluster from database of hanami
    match meta_cluster_table::delete_meta_cluster(&cluster_uuid, &context) {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    Ok(NoContent)
}
