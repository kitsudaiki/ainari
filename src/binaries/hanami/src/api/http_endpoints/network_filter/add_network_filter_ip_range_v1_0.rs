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
    summary = "Add network_filter ip-ranges",
    description = r###"Add ip-ranges to the packet filter of one direction of a virtual_machine.

The include-list starts out empty, which means "every address is allowed". The first entry flips
that around: from then on only packets are carried, whose address is named by one of the ranges.
For the `ingress` filter this is the source address of the traffic towards the virtual_machine,
for the `egress` filter the destination address of the traffic sent by the virtual_machine.
Entries may be written as a single address, as a subnet in CIDR notation or as an explicit range
`first-last`, and adding one that is already present is a no-op rather than an error.

The change is forwarded to the torii of the host of the virtual_machine."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 500
)]
pub async fn add_network_filter_ip_range(
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
    let filter_resp = network_filter_clients::add_filter_ip_range(
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
