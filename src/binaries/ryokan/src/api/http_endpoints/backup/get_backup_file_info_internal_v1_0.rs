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
use uuid::Uuid;
use validator::Validate;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::error::AinariError;

#[api_operation(
    tag = "backup",
    summary = "Get info of file in backup",
    description = r###"Get the size and the sha256 of a file within the package of a backup.

It is used to write the checksums of all files into the manifest, before the package is
finalized, and to check them again, before a backup is restored. So it is allowed in the states
`CREATING` and `READY`. The query-parameter `path` is relative to the directory of the package.
Large files, like the disks, take their time, because their whole content is read by the onsen.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn get_backup_file_info_internal(
    backup_uuid: Path<Uuid>,
    query: Query<BackupFileQuery>,
    context: UserContext,
) -> Result<Json<BackupFileInfoResp>, ErrorResponse> {
    query
        .validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    super::check_package_path(&query.path)?;

    let backup = super::get_backup_entry(&backup_uuid, &context)?;
    let status = super::backup_status_of(&backup)?;
    if !allows_file_info(status) {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' is in state '{status}', but reading a file-info requires the \
             state 'CREATING' or 'READY'.",
            backup.uuid
        )));
    }
    let host = super::get_backup_host(&backup, &context)?;

    let file_path = format!("{}/{}", backup.directory_path, query.path);
    let info = onsen_file_transfer::get_file_info(&host.address, &file_path)
        .await
        .map_err(|e| match e {
            AinariError::NotFound(_) => ErrorResponse::NotFound(format!(
                "File '{}' not found in backup with UUID '{}'.",
                query.path, *backup_uuid
            )),
            e => map_ainari_error_to_api_response(e, Some(&context)),
        })?;

    Ok(Json(BackupFileInfoResp {
        path: query.path.clone(),
        size: info.size,
        sha256: info.sha256,
    }))
}

/// Checks, if the files of a package can be checked in its state.
///
/// A package in the state `DELETING` or `ERROR` is incomplete, so its files are not checked
/// anymore.
fn allows_file_info(status: BackupStatus) -> bool {
    matches!(status, BackupStatus::Creating | BackupStatus::Ready)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allows_file_info() {
        assert!(allows_file_info(BackupStatus::Creating));
        assert!(allows_file_info(BackupStatus::Ready));
        assert!(!allows_file_info(BackupStatus::Deleting));
        assert!(!allows_file_info(BackupStatus::Error));
    }
}
