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

use actix_web::web::{Json, Path};
use apistos::api_operation;

use crate::core::network_filter::to_network_filter_resp;
use crate::database::network_filter_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_filter",
    summary = "Get network_filter",
    description = r###"Get the packet filter of one direction of a virtual_machine from the database.

A direction without a filter is unrestricted and reported as not found."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_network_filter(
    path: Path<NetworkFilterPath>,
    context: UserContext,
) -> Result<Json<NetworkFilterResp>, ErrorResponse> {
    let NetworkFilterPath {
        virtual_machine_uuid,
        direction,
    } = path.into_inner();

    let entry =
        network_filter_table::get_network_filter(&virtual_machine_uuid, direction, &context)
            .map_err(|e| {
                map_db_uuid_get_delete_error(
                    &format!("{direction} network_filter of virtual_machine"),
                    &virtual_machine_uuid,
                    e,
                )
            })?;

    Ok(Json(to_network_filter_resp(entry)?))
}
