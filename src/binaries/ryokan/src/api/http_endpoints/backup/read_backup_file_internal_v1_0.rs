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

use actix_web::web::Path;
use actix_web::web::{Json, Query};
use apistos::api_operation;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use uuid::Uuid;
use validator::Validate;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "backup",
    summary = "Read file from backup",
    description = r###"Read a file, like the metadata or the manifest, from the package of a backup.

Files can only be read, after the package was finalized and is in the state `READY`. The
query-parameter `path` is relative to the directory of the package. The file is returned
base64-encoded and can have at most 16 MiB.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn read_backup_file_internal(
    backup_uuid: Path<Uuid>,
    query: Query<BackupFileQuery>,
    context: UserContext,
) -> Result<Json<BackupFileResp>, ErrorResponse> {
    query
        .validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    super::check_package_path(&query.path)?;

    let backup = super::get_backup_entry(&backup_uuid, &context)?;
    super::check_backup_status(&backup, BackupStatus::Ready, "reading a file")?;
    let host = super::get_backup_host(&backup, &context)?;

    let file_path = format!("{}/{}", backup.directory_path, query.path);
    let content =
        onsen_file_transfer::download_data(&host.address, &file_path, MAX_BACKUP_FILE_SIZE)
            .await
            .map_err(|e| match e {
                AinariError::NotFound(_) => ErrorResponse::NotFound(format!(
                    "File '{}' not found in backup with UUID '{}'.",
                    query.path, *backup_uuid
                )),
                e => map_ainari_error_to_api_response(e, Some(&context)),
            })?;

    Ok(Json(BackupFileResp {
        path: query.path.clone(),
        content: BASE64.encode(content),
    }))
}
