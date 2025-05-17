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
use crate::core::cluster_handler;

use hanami_common::error::HanamiError;

use super::cluster_structs::{ClusterCreateReq, ClusterResp};

#[api_operation(
    tag = "cluster",
    summary = "Create new cluster",
    description = r###"Create new cluster based on a cluster-template."###,
    error_code = 400,
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
            let msg = format!("Invalid input: {}", e);
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
            let msg = format!("Failed to add cluster with UUID '{cluster_uuid}' to database.");
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // get new created cluster from database to get addtional information
    let cluster_data: cluster_table::ClusterEntry = match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(cluster_data) => cluster_data,
        Err(_) => 
        {
            let msg = format!("Failed to get cluster with ID '{cluster_uuid}' from database, even the cluster should exist.");
            error!("{}", msg);
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let resp = ClusterResp {
        uuid: cluster_uuid.clone(),
        name: cluster_data.name.clone(),
        template: cluster_data.template.clone(),
        created_by: cluster_data.created_by.clone(),
        created_at: cluster_data.created_at.clone(),
        updated_by: cluster_data.updated_by.clone(),
        updated_at: cluster_data.updated_at.clone(),
    };

    return Ok(CreatedJson(resp));
}
