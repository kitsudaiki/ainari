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
    summary = "Update status of image (internal)",
    description = r###"Set the status of an image, optionally together with type and size of its
file.

Sakura uses this for snapshots, which are registered before their file exists: the status is set to
`UPLOADING` together with type and size of the file, before the encrypted file is uploaded into the
onsen, and to `READY`, when the snapshot is complete. Type and size are measured on the unencrypted
file and have to be provided together.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn update_image_internal(
    body: Json<ImageUpdateInternalReq>,
    image_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<Json<ImageInternalResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let file_info = match (&body.file_type, body.file_size) {
        (Some(file_type), Some(file_size)) => {
            let file_size = i64::try_from(file_size)
                .map_err(|_| ErrorResponse::BadRequest("File-size is too big.".to_string()))?;
            Some((file_type.as_str(), file_size))
        }
        (None, None) => None,
        _ => {
            return Err(ErrorResponse::BadRequest(
                "'file_type' and 'file_size' have to be provided together.".to_string(),
            ));
        }
    };

    image_table::update_image_status(&image_uuid, &body.image_status, file_info, &context)
        .map_err(|e| map_db_uuid_get_delete_error("image", &image_uuid, e, Some(&context)))?;

    let image_data = image_table::get_image(&image_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("image", &image_uuid, e, Some(&context)))?;

    let image_status = super::image_status_of(&image_data)?;
    let resp = ImageInternalResp {
        uuid: *image_uuid,
        name: image_data.name,
        onsen_address: image_data.onsen_address,
        file_path: image_data.file_path,
        file_type: image_data.file_type,
        file_size: image_data.file_size as u64,
        image_status,
        is_snapshot: image_data.is_snapshot,
        secret_uuid: image_data.secret_uuid,
        created_by: image_data.created_by,
        created_at: image_data.created_at,
        updated_by: image_data.updated_by,
        updated_at: image_data.updated_at,
    };

    Ok(Json(resp))
}
