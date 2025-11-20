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

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::core::cluster_handler;
use crate::database::cluster_table;

use ainari_api::common_functions::map_ainari_error_to_api_response;
use ainari_api::common_functions::map_db_uuid_get_delete_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "cluster",
    summary = "Delete cluster",
    description = r###"Delete a cluster from the database and core."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_cluster_internal(
    cluster_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    // delete cluster from database
    cluster_table::delete_cluster(&cluster_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("cluster", &cluster_uuid, e))?;

    // delete cluster from core
    let mut cluster_handle = cluster_handler::CLUSTER_HANDLER
        .write()
        .expect("mutex poisoned");
    cluster_handle
        .delete_cluster(&cluster_uuid)
        .map_err(map_ainari_error_to_api_response)?;

    Ok(NoContent)
}
