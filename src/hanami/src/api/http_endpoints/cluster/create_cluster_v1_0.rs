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

use apistos::actix::CreatedJson;
use actix_web::web::Json;
use apistos::api_operation;
use log::error;
use uuid::Uuid;

use crate::api::user_context::UserContext;
use crate::api::errors::ErrorResponse;
use crate::database::cluster_table;

use hanami_core::cluster_handler;
use hanami_core::cluster::Cluster;
use hanami_common::error::HanamiError;

use super::cluster_structs::{ClusterCreateReq, ClusterResp};

#[api_operation(
    tag = "cluster",
    summary = "Create new cluster",
    description = r###"Create new cluster based on a cluster-template."###,
    error_code = 401,
    error_code = 500
)]
pub async fn create_cluster(body: Json<ClusterCreateReq>, context: UserContext) -> Result<CreatedJson<ClusterResp>, ErrorResponse> {
    let cluster_uuid = Uuid::new_v4();

    // parse cluster-template and create cluster from it
    let mut cluster_handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    match cluster_handle.create_cluster(cluster_uuid.clone(), body.name.clone(), body.template.clone()) {
        Ok(_) => {},
        Err(HanamiError::InputError(e)) => {
            let msg = format!("{}", e);
            return Err(ErrorResponse::BadRequest(msg));
        },
        Err(HanamiError::Error(e)) => {
            error!("{}", e);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // add new cluster to database
    match cluster_table::add_new_cluster(
        &cluster_uuid, 
        &body.name, 
        &body.template, 
        &context) 
    {
        Ok(_) => {},
        Err(_) => {
            let msg = format!("Failed to add cluster with UUID '{}' to database.", cluster_uuid);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created cluster from database to get addtional information
    match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(cluster) => {
            let resp = ClusterResp {
                uuid: cluster_uuid.clone(),
                name: cluster.name.clone(),
                template: cluster.template.clone(),
                created_by: cluster.created_by.clone(),
                created_at: cluster.created_at.clone(),
                updated_by: cluster.updated_by.clone(),
                updated_at: cluster.updated_at.clone(),
            };
        
            return Ok(CreatedJson(resp));
        },
        Err(_) => 
        {
            let msg = format!("Failed to get cluster with ID '{}' from database, even the cluster should exist.", cluster_uuid);
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
}
