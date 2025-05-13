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

use std::str::FromStr;

use apistos::actix::CreatedJson;
use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;
use log::error;
use uuid::Uuid;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::cluster_table;
use crate::core::cluster_handler;

use hanami_common::enums;

use super::cluster_structs::ClusterMode;
use super::cluster_structs::{ClusterModeSetReq, ClusterResp};

#[api_operation(
    tag = "cluster",
    summary = "Switch cluster-mode",
    description = r###"Switch cluster-mode between task-mode and direct-mode."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn switch_mode(cluster_uuid: Path<Uuid>, body: Json<ClusterModeSetReq>, context: UserContext) -> Result<CreatedJson<ClusterResp>, ErrorResponse> {

    // check if cluster exist
    match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{}' not found.", cluster_uuid);
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // get cluster-handle
    let mut cluster_handler = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    match cluster_handler.get(&cluster_uuid) {
        Some(cluster_handle) => {
            cluster_handle.set_mode(&body.mode);
        },
        None => return Err(ErrorResponse::InternalError("".to_string()))
    };

    // get cluster again from datbase
    let cluster_data: cluster_table::ClusterEntry = match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(cluster_data) => cluster_data,
        Err(_) => 
        {
            let msg = format!("Failed to get cluster with ID '{cluster_uuid}' from database, even the cluster should exist.");
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // convert cluster-mode
    let cluster_mode = match ClusterMode::from_str(cluster_data.mode.as_str()) {
        Ok(cluster_mode) => cluster_mode,
        Err(()) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let resp = ClusterResp {
        uuid: cluster_uuid.clone(),
        name: cluster_data.name.clone(),
        template: cluster_data.template.clone(),
        mode: cluster_mode,
        created_by: cluster_data.created_by.clone(),
        created_at: cluster_data.created_at.clone(),
        updated_by: cluster_data.updated_by.clone(),
        updated_at: cluster_data.updated_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
