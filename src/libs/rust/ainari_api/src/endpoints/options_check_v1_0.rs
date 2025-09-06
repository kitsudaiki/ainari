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

use actix_web::{HttpResponse, Responder, http::header};
use apistos::api_operation;

#[api_operation(
    tag = "version",
    summary = "Empty endpoint for options preflight-checks",
    description = r###"Empty endpoint for options preflight-checks."###,
    error_code = 500
)]
pub async fn options_check() -> impl Responder {
    return HttpResponse::Ok()
        .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"))
        // TODO: provide really only the avaiable types
        .insert_header((
            header::ACCESS_CONTROL_ALLOW_METHODS,
            "GET, POST, PUT, DELETE, OPTIONS",
        ))
        .insert_header((
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            "Authorization, Content-Type",
        ))
        .finish();
}
