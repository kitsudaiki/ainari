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

//! Clients of the internal endpoints of a storage-pool (onsen), which manage the volumes within
//! its storage-backend and inform about the backend itself.
//!
//! A volume is addressed by its name within the backend (`backend_id`).

use awc::ClientRequest;
use awc::http::StatusCode;
use uuid::Uuid;

use ainari_api_structs::storage_structs::*;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::{handle_empty_response, handle_response, prepare_client};

/// Endpoint of the volumes within the storage-backend.
const VOLUME_ENDPOINT: &str = "/v1alpha/volume";
/// Endpoint of the information about the storage-backend.
const BACKEND_ENDPOINT: &str = "/v1alpha/backend/internal";

/// Adds the token of the user and the internal API-key to a request.
fn authorized(request: ClientRequest, token: &str, internal_api_key: &Secret) -> ClientRequest {
    request
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
}

/// Creates an empty volume within the storage-backend.
///
/// Creating a volume again with the same size is no error, so a failed request can be repeated.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `backend_id` - Name of the volume within the backend.
/// * `size` - Size of the volume in bytes, a multiple of 512.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<VolumeInfoResp, AinariError>` - The new volume, `Conflict`, if a volume with another
///   size already exists or the backend doesn't support volumes, otherwise an AinariError.
pub async fn create_volume(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    backend_id: &Uuid,
    size: u64,
    insecure_client: bool,
) -> Result<VolumeInfoResp, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let body = VolumeCreateInternalReq {
        backend_id: *backend_id,
        size,
    };
    let request = authorized(
        client.post(format!("{onsen_address}{VOLUME_ENDPOINT}/internal")),
        token,
        internal_api_key,
    );
    handle_response(
        request.send_json(&body).await,
        "volume",
        &backend_id.to_string(),
    )
    .await
}

/// Requests the information about a volume within the storage-backend, which also reports the
/// state of a copy into the volume.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `backend_id` - Name of the volume within the backend.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<VolumeInfoResp, AinariError>` - The volume, `NotFound`, if it doesn't exist,
///   otherwise an AinariError.
pub async fn get_volume_info(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    backend_id: &Uuid,
    insecure_client: bool,
) -> Result<VolumeInfoResp, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.get(format!(
            "{onsen_address}{VOLUME_ENDPOINT}/{backend_id}/internal"
        )),
        token,
        internal_api_key,
    );
    handle_response(request.send().await, "volume", &backend_id.to_string()).await
}

/// Grows a volume within the storage-backend.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `backend_id` - Name of the volume within the backend.
/// * `size` - New size of the volume in bytes, which is not smaller than the current size.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<VolumeInfoResp, AinariError>` - The resized volume, `InvalidInput`, if the volume
///   would shrink, `NotFound`, if it doesn't exist, otherwise an AinariError.
pub async fn resize_volume(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    backend_id: &Uuid,
    size: u64,
    insecure_client: bool,
) -> Result<VolumeInfoResp, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let body = VolumeResizeInternalReq { size };
    let request = authorized(
        client.put(format!(
            "{onsen_address}{VOLUME_ENDPOINT}/{backend_id}/size/internal"
        )),
        token,
        internal_api_key,
    );
    handle_response(
        request.send_json(&body).await,
        "volume",
        &backend_id.to_string(),
    )
    .await
}

/// Deletes a volume from the storage-backend.
///
/// Deleting a not existing volume is no error.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `backend_id` - Name of the volume within the backend.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the deletion is successful, otherwise an AinariError.
pub async fn delete_volume(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    backend_id: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.delete(format!(
            "{onsen_address}{VOLUME_ENDPOINT}/{backend_id}/internal"
        )),
        token,
        internal_api_key,
    );
    handle_empty_response(request.send().await, "volume", &backend_id.to_string()).await
}

/// Starts the copy of a whole volume into a new volume within the same storage-backend.
///
/// The copy runs in the background. Its state is reported by `get_volume_info` of the target.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `source_backend_id` - Name of the volume, which is copied.
/// * `target_backend_id` - Name of the new volume, which gets the copy.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<(), AinariError>` - Ok if the copy was started, `NotFound`, if the source doesn't
///   exist, `Conflict`, if the target already exists, otherwise an AinariError.
pub async fn copy_volume(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    source_backend_id: &Uuid,
    target_backend_id: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let body = VolumeCopyInternalReq {
        target_backend_id: *target_backend_id,
    };
    let request = authorized(
        client.post(format!(
            "{onsen_address}{VOLUME_ENDPOINT}/{source_backend_id}/copy/internal"
        )),
        token,
        internal_api_key,
    );
    let response = request.send_json(&body).await;

    // the copy is only started, so the storage answers with `202 Accepted`
    if matches!(&response, Ok(resp) if resp.status() == StatusCode::ACCEPTED) {
        return Ok(());
    }
    handle_empty_response(response, "volume", &source_backend_id.to_string()).await
}

/// Requests the information, how a sakura-host connects to a volume.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `backend_id` - Name of the volume within the backend.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<VolumeConnection, AinariError>` - The connection, `NotFound`, if the volume doesn't
///   exist, `Conflict`, if the backend doesn't support volumes, otherwise an AinariError.
pub async fn get_volume_connection(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    backend_id: &Uuid,
    insecure_client: bool,
) -> Result<VolumeConnection, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.get(format!(
            "{onsen_address}{VOLUME_ENDPOINT}/{backend_id}/connection/internal"
        )),
        token,
        internal_api_key,
    );
    handle_response(
        request.send().await,
        "volume-connection",
        &backend_id.to_string(),
    )
    .await
}

/// Requests the information about the storage-backend of an onsen-host.
///
/// # Arguments
///
/// * `onsen_address` - The address of the storage.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// * `Result<StorageBackendResp, AinariError>` - Type, volume-support and capacity of the
///   backend, otherwise an AinariError.
pub async fn get_storage_backend(
    onsen_address: &str,
    token: &str,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<StorageBackendResp, AinariError> {
    let client = prepare_client(onsen_address, insecure_client);
    let request = authorized(
        client.get(format!("{onsen_address}{BACKEND_ENDPOINT}")),
        token,
        internal_api_key,
    );
    handle_response(request.send().await, "storage-backend", "").await
}
