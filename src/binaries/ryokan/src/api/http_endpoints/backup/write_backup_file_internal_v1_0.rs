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
use apistos::actix::NoContent;
use apistos::api_operation;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use uuid::Uuid;
use validator::Validate;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;

#[api_operation(
    tag = "backup",
    summary = "Write file into backup",
    description = r###"Write a file, like the metadata or the manifest, into the package of a
backup. An existing file with the same path is overwritten.

Files can only be written, while the backup is in the state `CREATING`. The `path` is relative to
the directory of the package and the base64-decoded `content` can have at most 16 MiB.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn write_backup_file_internal(
    backup_uuid: Path<Uuid>,
    body: Json<BackupFileWriteReq>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    super::check_package_path(&body.path)?;
    if context.is_read_only() {
        return Err(permission_denied_response());
    }

    let content = BASE64
        .decode(&body.content)
        .map_err(|e| ErrorResponse::BadRequest(format!("Content is not base64-encoded: {e}")))?;
    if content.len() > MAX_BACKUP_FILE_SIZE {
        return Err(ErrorResponse::BadRequest(format!(
            "Content is larger than {MAX_BACKUP_FILE_SIZE} bytes."
        )));
    }

    let backup = super::get_backup_entry(&backup_uuid, &context)?;
    super::check_backup_status(&backup, BackupStatus::Creating, "writing a file")?;
    let host = super::get_backup_host(&backup, &context)?;

    let file_path = format!("{}/{}", backup.directory_path, body.path);
    onsen_file_transfer::upload_data(&host.address, &file_path, content)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    Ok(NoContent)
}
