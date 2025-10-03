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
use apistos::api_operation;
use uuid::Uuid;

use crate::database::cluster_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::cluster_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "cluster",
    summary = "List cluster",
    description = r###"List basic information of all cluster from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_cluster(context: UserContext) -> Result<Json<ClusterListResp>, ErrorResponse> {
    let clusters = match cluster_table::list_clusters(&context) {
        Ok(clusters) => clusters,
        Err(e) => {
            log::error!("Failed to get list of clusters form database: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    let mut resp = ClusterListResp {
        clusters: Vec::new(),
    };

    for cluster in clusters {
        // parse-uuid-string coming from the database
        let uuid = match Uuid::parse_str(&cluster.uuid) {
            Ok(uuid) => uuid,
            Err(e) => {
                log::error!("Failed to convert cluster-uuid with error: '{e}'");
                return Err(ErrorResponse::InternalError("".to_string()));
            }
        };

        let obj = ClusterBasicResp {
            uuid,
            name: cluster.name.clone(),
        };

        resp.clusters.push(obj);
    }

    Ok(Json(resp))
}
