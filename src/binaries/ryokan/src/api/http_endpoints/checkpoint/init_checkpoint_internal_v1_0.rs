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

use crate::config;
use crate::database::checkpoint_table;
use crate::onsen_functions::select_onsen;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::checkpoint_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;

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
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let name = &body.name;
    let checkpoint_uuid = &body.uuid.clone();
    let file_path_str: String = format!("checkpoints/{}", checkpoint_uuid);

    check_checkpoint_quota(&context).await?;

    let selected_onsen = select_onsen(&context)?;

    let (secret_uuid, _) = super::super::generate_new_key(checkpoint_uuid, &context).await?;

    checkpoint_table::add_new_checkpoint(
        checkpoint_uuid,
        name,
        &selected_onsen.address,
        &file_path_str,
        &secret_uuid,
        &context,
    )
    .map_err(|e| {
        log::error!("Failed to add checkpoint to database: {e}");
        ErrorResponse::InternalError("Internal error".to_string())
    })?;

    let checkpoint = checkpoint_table::get_checkpoint(checkpoint_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("checkpoint", checkpoint_uuid, e))?;

    let secret_uuid = convert_uuid(&checkpoint.secret_uuid)?;
    let resp = CheckpointInternalResp {
        uuid: *checkpoint_uuid,
        name: checkpoint.name,
        onsen_address: checkpoint.onsen_address,
        file_path: checkpoint.file_path,
        secret_uuid,
        created_by: checkpoint.created_by,
        created_at: checkpoint.created_at,
        updated_by: checkpoint.updated_by,
        updated_at: checkpoint.updated_at,
    };

    Ok(CreatedJson(resp))
}

async fn check_checkpoint_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of checkpoints of the user
    let current_number_of_checkpoints =
        checkpoint_table::count_checkpoints(context).map_err(|e| {
            log::error!("Failed to count checkpoints in database.: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    // check the maximum number of checkpoints defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // check if quota is already exceeded
    if current_number_of_checkpoints as i64 >= quota.max_checkpoint as i64 {
        return Err(ErrorResponse::Conflict(
            "Maximum number of checkpoints exceeded.".to_string(),
        ));
    }

    Ok(())
}
