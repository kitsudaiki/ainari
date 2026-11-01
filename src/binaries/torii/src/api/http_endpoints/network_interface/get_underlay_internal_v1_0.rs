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

use crate::config::CONFIG;
use crate::core::utils::get_local_ip;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_interface_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "network_interface",
    summary = "Get underlay",
    description = r###"Get the underlay of this gateway, over which it exchanges the traffic of the
virtual machines with the other gateways.

hanami asks every gateway for the address of its underlay, instead of deriving it from the address
of its api, so the underlay can be a network of its own. The other gateways send the traffic of
the virtual machines on this host to this address, and it is the identity of the gateway within
the MLS-groups of the networks.

The address is empty, if the gateway has no underlay, like the single gateway of a single-node
setup, or if the interface of the underlay has no address yet."###,
    error_code = 401,
    error_code = 500
)]
pub async fn get_underlay_internal(
    _context: UserContext,
) -> Result<Json<UnderlayResp>, ErrorResponse> {
    let iface_name = CONFIG.network.underlay_iface.clone();
    let address = get_local_ip(&iface_name);

    Ok(Json(UnderlayResp {
        iface_name,
        address,
    }))
}
