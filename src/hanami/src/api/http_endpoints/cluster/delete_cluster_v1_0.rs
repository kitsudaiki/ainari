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
use apistos::api_operation;
use actix_web::web::Path;
use uuid::Uuid;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::cluster_table;

use hanami_common::enums;
use hanami_core::cluster_handler;

#[api_operation(
    tag = "cluster",
    summary = "Delete cluster",
    description = r###"Delete a cluster from the database and core."###,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_cluster(cluster_uuid: Path<Uuid>, context: UserContext) -> Result<NoContent, ErrorResponse> {
    // delete cluster from database
    match cluster_table::delete_cluster(&cluster_uuid, &context) {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{}' not found.", cluster_uuid);
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    // delete cluster from core
    let mut cluster_handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    if cluster_handle.delete(&cluster_uuid) == false {
        return Err(ErrorResponse::InternalError("".to_string()));
    }

    Ok(NoContent)   
}