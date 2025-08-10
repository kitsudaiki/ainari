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
use std::collections::HashMap;
use validator::Validate;
use std::sync::Arc;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::cluster_table;
use crate::core::cluster_handler;

use ainari_common::enums;
use ainari_structs::cluster_structs::{ClusterRequestReq, ClusterRequestResp};

#[api_operation(
    tag = "cluster",
    summary = "Get cluster",
    description = r###"Get information of a cluster from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn request_cluster(body: Json<ClusterRequestReq>, cluster_uuid: Path<Uuid>, context: UserContext) -> Result<Json<ClusterRequestResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {}", e);
            return Err(ErrorResponse::BadRequest(msg));
        },
    };

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

    // get cluster-interface
    let cluster_handler = cluster_handler::CLUSTER_HANDLER.read().unwrap();
    let cluster_interface_mutex = if let Some(c) = cluster_handler.get_cluster_interface(&cluster_uuid) {
        Arc::clone(&c)
    } else {
        return Err(ErrorResponse::InternalError("".to_string()));
    };
    drop(cluster_handler);

    let mut resp = ClusterRequestResp {
        outputs: HashMap::new(),
    };

    for hexagon_name in &body.outputs {
        resp.outputs.insert(hexagon_name.clone(), Vec::new());
    }

    // run request-process in cluster
    let mut cluster_interface = cluster_interface_mutex.lock().unwrap();
    match cluster_interface.request(&body.inputs, &mut resp.outputs) {
        Ok(()) => {},
        Err(msg) => {
            return Err(ErrorResponse::NotFound(msg));
        }
    }

    Ok(Json(resp))   
}