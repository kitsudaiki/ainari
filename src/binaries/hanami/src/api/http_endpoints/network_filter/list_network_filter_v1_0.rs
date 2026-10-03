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

use crate::core::network_filter::to_network_filter_resp;
use crate::database::network_filter_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "List network_filters",
    description = r###"List the packet filters of all virtual_machines from the database.

Directions of virtual_machines without a filter are unrestricted and not listed."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_network_filter(
    context: UserContext,
) -> Result<Json<NetworkFilterListResp>, ErrorResponse> {
    // get network_filters from db
    let entries = network_filter_table::list_network_filters(&context)
        .map_err(|e| map_db_list_error("network_filters", e))?;

    let network_filters = entries
        .into_iter()
        .map(to_network_filter_resp)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(NetworkFilterListResp { network_filters }))
}
