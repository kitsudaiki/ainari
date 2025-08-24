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
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::core::cluster_handler;
use crate::database::cluster_table;

use ainari_common::enums;
use ainari_common::error::AinariError;
use ainari_structs::cluster_structs::ClusterTrainReq;

#[api_operation(
    tag = "cluster",
    summary = "Get cluster",
    description = r###"Get information of a cluster from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn train_cluster(
    body: Json<ClusterTrainReq>,
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    // check if cluster exist
    match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // get cluster-interface
    let cluster_handler = cluster_handler::CLUSTER_HANDLER
        .read()
        .expect("mutex poisoned");
    let cluster_interface_mutex = match cluster_handler.get_cluster_interface(&cluster_uuid) {
        Ok(cluster_interface_mutex) => cluster_interface_mutex,
        Err(AinariError::InvalidInput(msg)) => {
            let msg = format!("Invalid input: {msg}");
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };
    drop(cluster_handler);

    // run train-process in cluster
    let mut cluster_interface = cluster_interface_mutex.lock().expect("mutex poisoned");
    match cluster_interface.train(&body.inputs, &body.outputs) {
        Ok(()) => {}
        Err(AinariError::InvalidInput(msg)) => {
            let msg = format!("Invalid input: {msg}");
            return Err(ErrorResponse::BadRequest(msg));
        }
        Err(AinariError::Error(msg)) => {
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    }

    Ok(NoContent)
}
