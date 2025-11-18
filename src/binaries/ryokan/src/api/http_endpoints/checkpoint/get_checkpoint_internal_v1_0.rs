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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::checkpoint_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "checkpoint",
    summary = "Get checkpoint (internal)",
    description = r###"Get information of a checkpoint from the database with additional information."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn get_checkpoint_internal(
    checkpoint_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<CheckpointInternalResp>, ErrorResponse> {
    let resp = super::get_checkpoint_internal(&checkpoint_uuid, &context)?;
    return Ok(Json(resp));
}
