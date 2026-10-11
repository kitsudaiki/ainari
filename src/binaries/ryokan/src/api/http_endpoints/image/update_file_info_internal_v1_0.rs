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
use uuid::Uuid;
use validator::Validate;

use crate::database::image_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::image_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "image",
    summary = "Set file-info of image (internal)",
    description = r###"Set type and size of the file of an image.

Sakura uses this for snapshots, which are registered before their file exists. Type and size are
measured on the unencrypted file.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn update_file_info(
    body: Json<ImageFileInfoReq>,
    image_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<ImageInternalResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let file_size = i64::try_from(body.file_size)
        .map_err(|_| ErrorResponse::BadRequest("File-size is too big.".to_string()))?;

    image_table::update_file_info(&image_uuid, &body.file_type, file_size, &context)
        .map_err(|e| map_db_uuid_get_delete_error("image", &image_uuid, e, Some(&context)))?;

    let image_data = image_table::get_image(&image_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("image", &image_uuid, e, Some(&context)))?;

    let resp = ImageInternalResp {
        uuid: *image_uuid,
        name: image_data.name,
        onsen_address: image_data.onsen_address,
        file_path: image_data.file_path,
        file_type: image_data.file_type,
        file_size: image_data.file_size as u64,
        is_snapshot: image_data.is_snapshot,
        secret_uuid: image_data.secret_uuid,
        created_by: image_data.created_by,
        created_at: image_data.created_at,
        updated_by: image_data.updated_by,
        updated_at: image_data.updated_at,
    };

    Ok(Json(resp))
}
