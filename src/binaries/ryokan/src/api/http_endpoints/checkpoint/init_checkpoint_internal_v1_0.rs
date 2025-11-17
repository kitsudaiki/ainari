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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::onsen_functions::select_onsen;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::checkpoint_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "checkpoint",
    summary = "Initialize new checkpoint",
    description = r###"Initialize  new checkpoint. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn init_checkpoint(
    body: Json<CheckpointCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<CheckpointInternalResp>, ErrorResponse> {
    // validate incoming json
    match body.validate() {
        Ok(_) => (),
        Err(e) => {
            let msg = format!("Invalid input: {e}");
            return Err(ErrorResponse::BadRequest(msg));
        }
    };

    let name = &body.name;
    let checkpoint_uuid = &body.uuid.clone();
    let file_path_str: String = format!("checkpoints/{}", checkpoint_uuid);

    super::check_checkpoint_quota(&context).await?;

    let selected_onsen = select_onsen(&context)?;

    super::add_checkpoint_to_database(
        checkpoint_uuid,
        name,
        &selected_onsen.address,
        &file_path_str,
        &context,
    )?;

    let resp = super::get_checkpoint_internal(checkpoint_uuid, &context)?;
    return Ok(CreatedJson(resp));
}
