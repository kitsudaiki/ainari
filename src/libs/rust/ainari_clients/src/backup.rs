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

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use uuid::Uuid;

use ainari_api_structs::backup_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_empty_response;
use crate::handle_response;
use crate::handle_response_with_limit;
use crate::prepare_client;

/// Creates a new backup in ryokan, whose package is stored on a backup-target.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier for the backup
/// * `project_id` - The project, which the backup belongs to
/// * `host_uuid` - Optional backup-target, where the package is stored
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the new backup in the state `CREATING`, or an `AinariError`.
pub async fn create_backup(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    project_id: &str,
    host_uuid: Option<&Uuid>,
    insecure_client: bool,
) -> Result<BackupResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/internal");

    let body = BackupCreateReq {
        uuid: *backup_uuid,
        project_id: project_id.to_owned(),
        host_uuid: host_uuid.copied(),
    };

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response::<BackupResp>(response, "backup", &backup_uuid.to_string()).await
}

/// Retrieves a backup from ryokan.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier of the backup
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the backup, or an `AinariError`.
pub async fn get_backup(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    insecure_client: bool,
) -> Result<BackupResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response::<BackupResp>(response, "backup", &backup_uuid.to_string()).await
}

/// Lists the backups in ryokan, which are accessible for the user.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `project_id` - Optional project to list only its backups
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the backups, or an `AinariError`.
pub async fn list_backups(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: Option<&str>,
    insecure_client: bool,
) -> Result<BackupListResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/internal");

    let query = BackupListQuery {
        project_id: project_id.map(str::to_owned),
    };
    let request = client
        .get(url)
        .query(&query)
        .map_err(|e| AinariError::internal(format!("Failed to build query of backups: {e}")))?;

    let response = request
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response::<BackupListResp>(response, "backup", "").await
}

/// Seals the package of a backup in ryokan, after all files were written into it.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier of the backup
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the backup in the state `READY`, or an `AinariError`. A package without
/// manifest is rejected with `Conflict`.
pub async fn finalize_backup(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    insecure_client: bool,
) -> Result<BackupResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/finalize/internal");

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response::<BackupResp>(response, "backup", &backup_uuid.to_string()).await
}

/// Deletes a backup in ryokan together with its whole package on the backup-target.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier of the backup
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing either an empty tuple (success) or an error
pub async fn delete_backup(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "backup", &backup_uuid.to_string()).await
}

/// Writes a file, like the metadata or the manifest, into the package of a backup in ryokan.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier of the backup
/// * `path` - Path of the file relative to the directory of the package
/// * `content` - Content of the file, at most `MAX_BACKUP_FILE_SIZE` bytes
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing either an empty tuple (success) or an error. A package, which is not in
/// the state `CREATING` anymore, is rejected with `Conflict`.
#[allow(clippy::too_many_arguments)]
pub async fn write_backup_file(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    path: &str,
    content: &[u8],
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/file/internal");

    let body = BackupFileWriteReq {
        path: path.to_owned(),
        content: BASE64.encode(content),
    };

    let response = client
        .put(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_empty_response(response, "backup", &backup_uuid.to_string()).await
}

/// Reads a file, like the metadata or the manifest, from the package of a backup in ryokan.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier of the backup
/// * `path` - Path of the file relative to the directory of the package
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the content of the file, or an `AinariError`. A package, which is not in
/// the state `READY`, is rejected with `Conflict`.
pub async fn read_backup_file(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    path: &str,
    insecure_client: bool,
) -> Result<Vec<u8>, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/file/internal");

    let query = BackupFileQuery {
        path: path.to_owned(),
    };
    let request = client
        .get(url)
        .query(&query)
        .map_err(|e| AinariError::internal(format!("Failed to build query of backup-file: {e}")))?;

    let response = request
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: BackupFileResp = handle_response_with_limit(
        response,
        "backup",
        &backup_uuid.to_string(),
        MAX_BACKUP_FILE_MESSAGE_SIZE,
    )
    .await?;

    BASE64.decode(resp.content).map_err(|e| {
        AinariError::internal(format!(
            "Content of file '{path}' of backup '{backup_uuid}' is not base64-encoded: {e}"
        ))
    })
}

/// Gets the size and the sha256 of a file within a backup.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The backup, which contains the file
/// * `path` - Path of the file relative to the directory of the backup
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the size and checksum of the file, or an `AinariError`.
pub async fn get_backup_file_info(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    path: &str,
    insecure_client: bool,
) -> Result<BackupFileInfoResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/file_info/internal");

    let query = BackupFileQuery {
        path: path.to_owned(),
    };
    let request = client
        .get(url)
        .query(&query)
        .map_err(|e| AinariError::internal(format!("Failed to build query of backup-file: {e}")))?;

    // the onsen reads the whole file for its checksum, which takes its time for large disks
    let response = request
        .timeout(std::time::Duration::from_secs(COPY_TIMEOUT_SECONDS))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response::<BackupFileInfoResp>(response, "backup", &backup_uuid.to_string()).await
}

/// Requests the target in ryokan, where the root-disk of a virtual_machine is uploaded into the
/// package of a backup.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier of the backup
/// * `vm_uuid` - The virtual_machine, whose root-disk is stored
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the address of the onsen and the path of the disk, or an `AinariError`.
pub async fn prepare_backup_disk(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    vm_uuid: &Uuid,
    insecure_client: bool,
) -> Result<BackupDiskResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/disk/internal");

    let body = BackupDiskReq { vm_uuid: *vm_uuid };

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response::<BackupDiskResp>(response, "backup", &backup_uuid.to_string()).await
}

/// Lists the packages on a backup-target, which are not registered as backup in ryokan.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token of an admin
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `host_uuid` - The backup-target to scan
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the found packages, or an `AinariError`.
pub async fn scan_backup_target(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    host_uuid: &Uuid,
    insecure_client: bool,
) -> Result<BackupScanResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup_target/{host_uuid}/scan/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response::<BackupScanResp>(response, "onsen-host", &host_uuid.to_string()).await
}

/// Registers a package, which was copied onto a backup-target from outside, as backup in ryokan.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token of an admin
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The UUID of the backup, which is the name of its directory
/// * `project_id` - The project, which the backup belongs to
/// * `host_uuid` - The backup-target, which stores the package
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the backup in the state `READY`, or an `AinariError`.
pub async fn register_backup(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    project_id: &str,
    host_uuid: &Uuid,
    insecure_client: bool,
) -> Result<BackupResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/register/internal");

    let body = BackupRegisterReq {
        uuid: *backup_uuid,
        project_id: project_id.to_owned(),
        host_uuid: *host_uuid,
    };

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response::<BackupResp>(response, "backup", &backup_uuid.to_string()).await
}

/// Copies a file between the package of a backup and the normal onsens over ryokan.
///
/// The file is streamed from the source-onsen to the target-onsen, without storing it in ryokan.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `backup_uuid` - The unique identifier of the backup
/// * `direction` - `IntoBackup` to copy the file of an image into the package, `OutOfBackup` to
///   copy a file of the package to a normal onsen
/// * `path` - Path of the file relative to the directory of the package
/// * `image_uuid` - The image, whose file is copied into the package, or the new UUID of the image,
///   under which the file is stored on the normal onsen
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the address of the target-onsen, the path and the size of the copy, or an
/// `AinariError`.
#[allow(clippy::too_many_arguments)]
pub async fn copy_backup_file(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    backup_uuid: &Uuid,
    direction: BackupCopyDirection,
    path: &str,
    image_uuid: &Uuid,
    insecure_client: bool,
) -> Result<BackupCopyResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/backup/{backup_uuid}/copy/internal");

    let body = BackupCopyReq {
        direction,
        path: path.to_owned(),
        image_uuid: *image_uuid,
    };

    // large files take their time, so the default timeout of the client is not enough
    let response = client
        .post(url)
        .timeout(std::time::Duration::from_secs(COPY_TIMEOUT_SECONDS))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response::<BackupCopyResp>(response, "backup", &backup_uuid.to_string()).await
}

/// Timeout of a copy of a file in seconds, which also covers large disks.
const COPY_TIMEOUT_SECONDS: u64 = 6 * 60 * 60;
