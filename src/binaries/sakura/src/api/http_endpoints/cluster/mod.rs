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

pub mod create_cluster_internal_v1_0;
pub mod delete_cluster_internal_v1_0;
pub mod get_cluster_internal_v1_0;
pub mod list_cluster_internal_v1_0;
pub mod request_cluster_v1_0;
pub mod train_cluster_v1_0;

pub mod task;

use uuid::Uuid;

use crate::database::cluster_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

pub fn get_cluster_from_database(
    cluster_uuid: &Uuid,
    context: &UserContext,
) -> Result<cluster_table::ClusterEntry, ErrorResponse> {
    let cluster = match cluster_table::get_cluster(cluster_uuid, context) {
        Ok(cluster) => cluster,
        Err(enums::DbError::InternalError) => {
            log::error!("Internal error while requesting cluster from database");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    Ok(cluster)
}
