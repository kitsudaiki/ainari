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

use uuid::Uuid;

use ainari_api_structs::snapshot_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Initializes a new snapshot in the system.
///
/// This function creates a new snapshot with the provided UUID and name.
/// It communicates with the Ryokan service to perform the snapshot creation.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `snapshot_uuid` - The unique identifier for the new snapshot
/// * `name` - The human-readable name for the snapshot
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the `SnapshotInternalResp` if successful, or an `AinariError` if the operation fails.
pub async fn init_snapshot(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    snapshot_uuid: &Uuid,
    name: &str,
    insecure_client: bool,
) -> Result<SnapshotInternalResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/snapshot/internal");

    let body = SnapshotCreateReq {
        uuid: *snapshot_uuid,
        name: name.to_owned(),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<SnapshotInternalResp, AinariError> =
        handle_response(response, "snapshot", "").await;
    resp
}

/// Retrieves an existing snapshot from the system.
///
/// This function fetches the details of a snapshot identified by its UUID.
/// It communicates with the Ryokan service to perform the snapshot retrieval.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `snapshot_uuid` - The unique identifier of the snapshot to retrieve
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the `SnapshotInternalResp` if successful, or an `AinariError` if the operation fails.
pub async fn get_snapshot(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    snapshot_uuid: &Uuid,
    insecure_client: bool,
) -> Result<SnapshotInternalResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/snapshot/{snapshot_uuid}/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<SnapshotInternalResp, AinariError> =
        handle_response(response, "snapshot", &snapshot_uuid.to_string()).await;
    resp
}
