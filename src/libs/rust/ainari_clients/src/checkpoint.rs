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

use ainari_api_structs::checkpoint_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Initializes a new checkpoint in the system.
///
/// This function creates a new checkpoint with the provided UUID and name.
/// It communicates with the Ryokan service to perform the checkpoint creation.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `checkpoint_uuid` - The unique identifier for the new checkpoint
/// * `name` - The human-readable name for the checkpoint
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the `CheckpointInternalResp` if successful, or an `AinariError` if the operation fails.
pub async fn init_checkpoint(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    checkpoint_uuid: &Uuid,
    name: &str,
    insecure_client: bool,
) -> Result<CheckpointInternalResp, AinariError> {
    // Clone the internal address from the endpoint configuration
    let address = ryokan_endpoint.internal_address.clone();
    // Prepare the HTTP client based on the address and security settings
    let client = prepare_client(&address, insecure_client);
    // Construct the API endpoint URL for checkpoint creation
    let url = format!("{address}/v1alpha/checkpoint/internal");

    // Create the request body with the provided checkpoint details
    let body = CheckpointCreateReq {
        uuid: *checkpoint_uuid,
        name: name.to_owned(),
    };
    // Serialize the request body to JSON
    let json_str = serde_json::to_string(&body).unwrap();

    // Send the POST request to create the checkpoint
    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    // Handle the response and return the result
    let resp: Result<CheckpointInternalResp, AinariError> =
        handle_response(response, "checkpoint", "").await;
    resp
}

/// Retrieves an existing checkpoint from the system.
///
/// This function fetches the details of a checkpoint identified by its UUID.
/// It communicates with the Ryokan service to perform the checkpoint retrieval.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `checkpoint_uuid` - The unique identifier of the checkpoint to retrieve
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the `CheckpointInternalResp` if successful, or an `AinariError` if the operation fails.
pub async fn get_checkpoint(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    checkpoint_uuid: &Uuid,
    insecure_client: bool,
) -> Result<CheckpointInternalResp, AinariError> {
    // Clone the internal address from the endpoint configuration
    let address = ryokan_endpoint.internal_address.clone();
    // Prepare the HTTP client based on the address and security settings
    let client = prepare_client(&address, insecure_client);
    // Construct the API endpoint URL for checkpoint retrieval
    let url = format!("{address}/v1alpha/checkpoint/{checkpoint_uuid}/internal");

    // Send the GET request to retrieve the checkpoint details
    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    // Handle the response and return the result
    let resp: Result<CheckpointInternalResp, AinariError> =
        handle_response(response, "checkpoint", &checkpoint_uuid.to_string()).await;
    resp
}
