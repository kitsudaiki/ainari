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

use crate::database::cluster_table;

use ainari_api::errors::ErrorResponse;
use ainari_api::structs::cluster_structs::ClusterResp;
use ainari_api::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "cluster",
    summary = "Get cluster",
    description = r###"Get information of a cluster from the database."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_cluster(
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<ClusterResp>, ErrorResponse> {
    let cluster_data = match cluster_table::get_cluster(&cluster_uuid, &context) {
        Ok(cluster_data) => cluster_data,
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Cluster with UUID '{cluster_uuid}' not found.");
            return Err(ErrorResponse::NotFound(msg));
        }
    };

    let resp = ClusterResp {
        uuid: *cluster_uuid,
        name: cluster_data.name.clone(),
        template: cluster_data.template.clone(),
        created_by: cluster_data.created_by.clone(),
        created_at: cluster_data.created_at.clone(),
        updated_by: cluster_data.updated_by.clone(),
        updated_at: cluster_data.updated_at.clone(),
    };

    return Ok(Json(resp));
}
