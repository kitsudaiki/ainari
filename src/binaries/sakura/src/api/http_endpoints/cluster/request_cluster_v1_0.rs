// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
use std::collections::HashMap;
use uuid::Uuid;
use validator::Validate;

use crate::core::cluster_handler;
use crate::database::cluster_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::cluster_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "cluster",
    summary = "Get cluster",
    description = r###"Get information of a cluster from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn request_cluster(
    body: Json<ClusterRequestReq>,
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<ClusterRequestResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // check if cluster exist
    cluster_table::get_cluster(&cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("cluster", &cluster_uuid, e))?;

    // get cluster-interface
    let cluster_handler = cluster_handler::CLUSTER_HANDLER
        .read()
        .expect("mutex poisoned");
    let cluster_interface_mutex = cluster_handler
        .get_cluster_interface(&cluster_uuid)
        .map_err(map_ainari_error_to_api_response)?;
    drop(cluster_handler);

    let mut resp = ClusterRequestResp {
        outputs: HashMap::new(),
    };

    for hexagon_name in &body.outputs {
        resp.outputs.insert(hexagon_name.clone(), Vec::new());
    }

    // run request-process in cluster
    let mut cluster_interface = cluster_interface_mutex.lock().expect("mutex poisoned");
    cluster_interface
        .request(&body.inputs, &mut resp.outputs)
        .map_err(map_ainari_error_to_api_response)?;

    Ok(Json(resp))
}
