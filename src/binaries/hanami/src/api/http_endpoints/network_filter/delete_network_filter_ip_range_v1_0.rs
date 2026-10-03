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
use validator::Validate;

use crate::config;
use crate::core::network_filter::{
    NETWORK_FILTER_LOCK, resolve_filter_target, store_network_filter, to_network_filter_resp,
};

use ainari_api::common_functions::map_ainari_error_to_api_response;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::network_filter as network_filter_clients;

#[api_operation(
    tag = "network_filter",
    summary = "Remove network_filter ip-ranges",
    description = r###"Remove ip-ranges from the packet filter of one direction of a virtual_machine.

An entry is identified by the addresses it covers, not by the way it was written down:
`10.0.0.0/24` and `10.0.0.0-10.0.0.255` remove the same entry. Removing the last range opens this
direction for every address again. A filter, whose include-lists are both empty, is removed."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_network_filter_ip_range(
    path: Path<NetworkFilterPath>,
    body: Json<FilterIpRangeReq>,
    context: UserContext,
) -> Result<Json<NetworkFilterResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    if body.ranges.is_empty() {
        return Err(ErrorResponse::BadRequest("No IP range given".to_string()));
    }

    let NetworkFilterPath {
        virtual_machine_uuid,
        direction,
    } = path.into_inner();

    let _lock = NETWORK_FILTER_LOCK.lock().await;
    let target = resolve_filter_target(&virtual_machine_uuid, &context).await?;

    // the change is applied by the torii of the host of the virtual_machine
    let filter_resp = network_filter_clients::delete_filter_ip_range(
        &target.torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        target.vni,
        &target.ip,
        direction,
        body.into_inner().ranges,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    let entry = store_network_filter(&virtual_machine_uuid, direction, &filter_resp, &context)?;

    Ok(Json(to_network_filter_resp(entry)?))
}
