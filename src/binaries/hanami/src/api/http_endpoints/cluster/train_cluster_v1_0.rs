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

use apistos::actix::NoContent;
use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::cluster_table;
use crate::core::cluster_handler;

use hanami_common::enums;
use hanami_structs::cluster_structs::{ClusterTrainReq};

#[api_operation(
    tag = "cluster",
    summary = "Get cluster",
    description = r###"Get information of a cluster from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn train_cluster(body: Json<ClusterTrainReq>, cluster_uuid: Path<Uuid>, context: UserContext) -> Result<NoContent, ErrorResponse> {
    // check if cluster exist
    match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // get cluster-handle
    let mut cluster_handler = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    let cluster_handle = match cluster_handler.get(&cluster_uuid) {
        Some(cluster_handle) => cluster_handle,
        None => return Err(ErrorResponse::InternalError("".to_string()))
    };

    // run train-process in cluster
    match cluster_handle.train(&body.inputs, &body.outputs) {
        Ok(()) => {},
        Err(msg) => {
            return Err(ErrorResponse::NotFound(msg));
        }
    }

    Ok(NoContent)   
}