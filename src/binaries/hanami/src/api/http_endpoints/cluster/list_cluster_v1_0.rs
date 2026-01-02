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
use apistos::api_operation;

use crate::config;
use crate::database::host_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::cluster_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::cluster as cluster_clients;

#[api_operation(
    tag = "cluster",
    summary = "List cluster",
    description = r###"List basic information of all cluster from the database."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_cluster(context: UserContext) -> Result<Json<ClusterListResp>, ErrorResponse> {
    let mut complete_resp = ClusterListResp {
        clusters: Vec::new(),
    };

    let host_list = host_table::list_hosts(&context).map_err(|e| map_db_list_error("hosts", e))?;

    for host in host_list {
        let cluster_list_resp = cluster_clients::list_cluster(
            &host.address,
            &context.token,
            &config::INTERNAL_API_KEY,
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response)?;

        for cluster in cluster_list_resp.clusters {
            complete_resp.clusters.push(cluster);
        }
    }

    Ok(Json(complete_resp))
}
