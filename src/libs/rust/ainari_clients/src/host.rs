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

use ainari_api_structs::host_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Registers a Sakura host with the Hanami service.
///
/// This function creates a new Sakura host entry in the Hanami service by sending a POST request
/// with the host details. The function handles the creation of the HTTP client, request body,
/// and response processing.
///
/// # Arguments
///
/// * `hanami_endpoint` - The endpoint configuration for the Hanami service
/// * `internal_api_key` - The API key for internal authentication
/// * `name` - The name of the Sakura host to register
/// * `sakura_address` - The address of the Sakura host
/// * `deleted_uuids` - List of UUIDs that have been deleted and need to be tracked
/// * `registration_key` - The registration key for the host
/// * `insecure_client` - Whether to create an insecure client (bypasses TLS verification)
///
/// # Returns
///
/// A `Result` containing either the `HostResp` from the server or an `AinariError` if the request fails.
pub async fn register_sakura_host(
    hanami_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    name: &str,
    sakura_address: &str,
    deleted_uuids: UuidList,
    registration_key: &Secret,
    insecure_client: bool,
) -> Result<HostResp, AinariError> {
    // Clone the internal address from the Hanami endpoint configuration
    let address = hanami_endpoint.internal_address.clone();

    // Prepare the HTTP client with the appropriate settings
    let client = prepare_client(&address, insecure_client);

    // Construct the URL for the host registration endpoint
    let url = format!("{address}/v1alpha/host/internal");

    // Create the request body with host details
    let body = HostCreateReq {
        name: name.to_owned(),
        host_address: sakura_address.to_owned(),
        deleted_uuids,
        registration_key: Secret::from(registration_key.reveal()),
    };

    // Serialize the request body to JSON
    let json_str = serde_json::to_string(&body).unwrap();

    // Send the POST request with the required headers
    let response = client
        .post(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    // Handle the response and return the result
    let resp: Result<HostResp, AinariError> = handle_response(response, "sakura-host", "").await;
    resp
}

/// Registers an Onsen host with the Ryokan service.
///
/// This function creates a new Onsen host entry in the Ryokan service by sending a POST request
/// with the host details. The function handles the creation of the HTTP client, request body,
/// and response processing.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `internal_api_key` - The API key for internal authentication
/// * `name` - The name of the Onsen host to register
/// * `onsen_address` - The address of the Onsen host
/// * `registration_key` - The registration key for the host
/// * `insecure_client` - Whether to create an insecure client (bypasses TLS verification)
///
/// # Returns
///
/// A `Result` containing either the `HostResp` from the server or an `AinariError` if the request fails.
pub async fn register_onsen_host(
    ryokan_endpoint: &ainari_config::Endpoint,
    internal_api_key: &Secret,
    name: &str,
    onsen_address: &str,
    registration_key: &Secret,
    insecure_client: bool,
) -> Result<HostResp, AinariError> {
    // Clone the internal address from the Ryokan endpoint configuration
    let address = ryokan_endpoint.internal_address.clone();

    // Prepare the HTTP client with the appropriate settings
    let client = prepare_client(&address, insecure_client);

    // Construct the URL for the host registration endpoint
    let url = format!("{address}/v1alpha/host/internal");

    // Create an empty UUID list as Onsen hosts typically don't have deleted UUIDs
    let empty_default_uuid_list = UuidList::default();

    // Create the request body with host details
    let body = HostCreateReq {
        name: name.to_owned(),
        host_address: onsen_address.to_owned(),
        deleted_uuids: empty_default_uuid_list,
        registration_key: Secret::from(registration_key.reveal()),
    };

    // Serialize the request body to JSON
    let json_str = serde_json::to_string(&body).unwrap();

    // Send the POST request with the required headers
    let response = client
        .post(url)
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    // Handle the response and return the result
    let resp: Result<HostResp, AinariError> = handle_response(response, "onsen-host", "").await;
    resp
}
