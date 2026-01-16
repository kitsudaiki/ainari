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

use crate::errors::ErrorResponse;
use ainari_api_structs::common_structs::ReadyResp;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "version",
    summary = "Get ready status",
    description = r###"Get status of the service endpoint."###,
    error_code = 500
)]
pub async fn get_ready_status(_: UserContext) -> Result<Json<ReadyResp>, ErrorResponse> {
    // Create a new ReadyResp instance indicating the API is ready
    let resp = ReadyResp { api: true };

    // Return the response wrapped in Json
    Ok(Json(resp))
}
