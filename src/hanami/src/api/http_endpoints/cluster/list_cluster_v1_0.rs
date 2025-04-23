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

use crate::api::errors::ErrorResponse;
use crate::api::user_context::UserContext;
use crate::database::cluster_table;

use super::cluster_structs::{ClusterBasicResp, ClusterListResp};

#[api_operation(
    tag = "cluster",
    summary = "List cluster",
    description = r###"List basic information of all cluster from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_cluster(context: UserContext) -> Result<Json<ClusterListResp>, ErrorResponse> {
    let clusters = cluster_table::list_clusters().unwrap();

    let mut resp = ClusterListResp {
        clusters: Vec::new(),
    };

    for cluster in clusters {
        let obj = ClusterBasicResp {
            uuid: cluster.uuid.clone(),
            name: cluster.name.clone(),
        };

        resp.clusters.push(obj); // fill the vector with objects
    }

    Ok(Json(resp))
}
