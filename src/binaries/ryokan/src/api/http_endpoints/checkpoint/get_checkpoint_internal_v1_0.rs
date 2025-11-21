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

use crate::database::checkpoint_table;

use ainari_api::common_functions::*;
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
    let checkpoint = checkpoint_table::get_checkpoint(&checkpoint_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("checkpoint", &checkpoint_uuid, e))?;

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

    Ok(Json(resp))
}
