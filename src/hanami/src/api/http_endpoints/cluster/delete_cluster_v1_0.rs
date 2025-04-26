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

use std::fmt::format;

use apistos::actix::NoContent;
use apistos::api_operation;
use actix_web::web::Path;
use uuid::Uuid;

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::cluster_table;

use hanami_common::enums;
use hanami_core::cluster_handler;
use hanami_core::cluster::Cluster;

#[api_operation(
    tag = "cluster",
    summary = "Delete cluster",
    description = r###"Delete a cluster from the database and core."###,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_cluster(uuid: Path<Uuid>, context: UserContext) -> Result<NoContent, ErrorResponse> {
    // check first in database
    match cluster_table::get_cluster(&uuid) {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };

    // delete cluster from core
    let mut cluster_handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    if cluster_handle.delete(&uuid) == false {
        return Err(ErrorResponse::InternalError("".to_string()));
    }
    
    // get new created cluster from database to get addtional information
    match cluster_table::delete_cluster(&uuid) {
        Ok(_) => {
            return Ok(NoContent);
        },
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        },
        Err(enums::DbError::NotFound) => {
            return Err(ErrorResponse::NotFound("".to_string()));
        }
    };
}