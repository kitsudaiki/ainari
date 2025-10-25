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
use ainari_api_structs::endpoints_structs::EndpointField;
use apistos::api_operation;

use crate::config;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::endpoints_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "endpoints",
    summary = "Get ednpoints",
    description = r###"Get addresses and ports of all endpoints."###,
    error_code = 400,
    error_code = 500
)]
pub async fn get_endpoints(_: UserContext) -> Result<Json<EndpontsResp>, ErrorResponse> {
    let enpoint_config = &config::CONFIG.endpoints;

    let response = EndpontsResp {
        hanami: EndpointField {
            public_address: enpoint_config.hanami.public_address.clone(),
            public_port: enpoint_config.hanami.public_port,
            internal_address: enpoint_config.hanami.internal_address.clone(),
            internal_port: enpoint_config.hanami.internal_port,
        },
        bento: EndpointField {
            public_address: enpoint_config.bento.public_address.clone(),
            public_port: enpoint_config.bento.public_port,
            internal_address: enpoint_config.bento.internal_address.clone(),
            internal_port: enpoint_config.bento.internal_port,
        },
    };

    return Ok(Json(response));
}
