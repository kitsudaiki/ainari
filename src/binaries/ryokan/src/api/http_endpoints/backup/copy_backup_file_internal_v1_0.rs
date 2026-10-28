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
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::database::image_table;
use crate::onsen_functions::select_onsen;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::image_structs::ImageStatus;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "backup",
    summary = "Copy file of backup",
    description = r###"Copy a file between the package of a backup and the normal onsens.

The file is streamed chunk by chunk from the source-onsen to the target-onsen, without storing it in
ryokan. If the copy fails, the incomplete copy is deleted again.

- `INTO_BACKUP`: the file of the `READY` image `image_uuid` is copied unchanged into the package
  under `path`. The package must be in the state `CREATING`.
- `OUT_OF_BACKUP`: the file `path` of the package is copied to a normal onsen under
  `images/<image_uuid>`, where it can be registered as image with this UUID afterwards. The UUID
  must not be used by any image yet and the package must be in the state `READY`, so it stays
  untouched.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn copy_backup_file_internal(
    backup_uuid: Path<Uuid>,
    body: Json<BackupCopyReq>,
    context: UserContext,
) -> Result<Json<BackupCopyResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    super::check_package_path(&body.path)?;
    // both directions write a new file
    if context.is_read_only() {
        return Err(permission_denied_response());
    }

    let backup = super::get_backup_entry(&backup_uuid, &context)?;
    let backup_host = super::get_backup_host(&backup, &context)?;
    let package_file = format!("{}/{}", backup.directory_path, body.path);

    let (source_address, source_path, target_address, target_path) = match body.direction {
        BackupCopyDirection::IntoBackup => {
            super::check_backup_status(&backup, BackupStatus::Creating, "copying a file into it")?;

            let image = image_table::get_image(&body.image_uuid, &context).map_err(|e| {
                map_db_uuid_get_delete_error("image", &body.image_uuid, e, Some(&context))
            })?;
            if image.image_status != ImageStatus::Ready.as_str() {
                return Err(ErrorResponse::Conflict(format!(
                    "Image with UUID '{}' is not ready.",
                    body.image_uuid
                )));
            }
            (
                image.onsen_address,
                image.file_path,
                backup_host.address,
                package_file,
            )
        }
        BackupCopyDirection::OutOfBackup => {
            super::check_backup_status(&backup, BackupStatus::Ready, "copying a file out of it")?;

            let is_taken = image_table::is_uuid_taken(&body.image_uuid)
                .map_err(|e| map_db_list_error("images", e, Some(&context)))?;
            if is_taken {
                return Err(ErrorResponse::Conflict(format!(
                    "Image with UUID '{}' already exists.",
                    body.image_uuid
                )));
            }
            let image_host = select_onsen(&context)?;
            (
                backup_host.address,
                package_file,
                image_host.address,
                format!("images/{}", body.image_uuid),
            )
        }
    };

    let size = onsen_file_transfer::copy_file(
        &source_address,
        &source_path,
        &target_address,
        &target_path,
    )
    .await
    .map_err(|e| match e {
        AinariError::NotFound(_) => ErrorResponse::NotFound(format!(
            "File '{source_path}' not found on onsen '{source_address}'."
        )),
        e => map_ainari_error_to_api_response(e, Some(&context)),
    })?;

    Ok(Json(BackupCopyResp {
        onsen_address: target_address,
        file_path: target_path,
        size,
    }))
}
