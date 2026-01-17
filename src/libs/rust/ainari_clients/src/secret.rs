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

use ainari_api_structs::secret_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;

use crate::handle_empty_response;
use crate::handle_response;
use crate::prepare_client;

/// Generates a new secret with the given name
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the Omamori service
/// * `token` - The authentication token for the API request
/// * `name` - The name to assign to the new secret
/// * `insecure_client` - Whether to create an insecure HTTP client (for testing purposes)
///
/// # Returns
///
/// A `Result` containing either the generated secret response or an error
///
/// # Errors
///
/// This function will return an error if:
/// - The client preparation fails
/// - The API request fails
/// - The response handling fails
pub async fn generate_secret(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    name: &str,
    insecure_client: bool,
) -> Result<SecretResp, AinariError> {
    // Clone the internal address from the endpoint configuration
    let address = omamori_endpoint.internal_address.clone();
    // Prepare the HTTP client based on the address and security requirements
    let client = prepare_client(&address, insecure_client);
    // Construct the URL for the secret generation endpoint
    let url = format!("{address}/v1alpha/secret/generate");

    // Create the request body with the provided secret name
    let body = SecretGenerateReq {
        name: name.to_owned(),
    };
    // Serialize the request body to JSON
    let json_str = serde_json::to_string(&body).unwrap();

    // Make the POST request to generate the secret
    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    // Handle the response and return the result
    let resp: Result<SecretResp, AinariError> = handle_response(response, "secret", "").await;
    resp
}

/// Retrieves the payload of an existing secret
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the Omamori service
/// * `token` - The authentication token for the API request
/// * `secret_uuid` - The UUID of the secret to retrieve
/// * `insecure_client` - Whether to create an insecure HTTP client (for testing purposes)
///
/// # Returns
///
/// A `Result` containing either the secret with payload response or an error
///
/// # Errors
///
/// This function will return an error if:
/// - The client preparation fails
/// - The API request fails
/// - The response handling fails
pub async fn get_secret_payload(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    secret_uuid: &Uuid,
    insecure_client: bool,
) -> Result<SecretWithPayloadResp, AinariError> {
    // Clone the internal address from the endpoint configuration
    let address = omamori_endpoint.internal_address.clone();
    // Prepare the HTTP client based on the address and security requirements
    let client = prepare_client(&address, insecure_client);
    // Construct the URL for the secret payload endpoint
    let url = format!("{address}/v1alpha/secret/{secret_uuid}/payload");

    // Make the GET request to retrieve the secret payload
    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    // Handle the response and return the result
    let resp: Result<SecretWithPayloadResp, AinariError> =
        handle_response(response, "secret", &secret_uuid.to_string()).await;
    resp
}

/// Deletes an existing secret
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the Omamori service
/// * `token` - The authentication token for the API request
/// * `secret_uuid` - The UUID of the secret to delete
/// * `insecure_client` - Whether to create an insecure HTTP client (for testing purposes)
///
/// # Returns
///
/// A `Result` containing either an empty tuple (success) or an error
///
/// # Errors
///
/// This function will return an error if:
/// - The client preparation fails
/// - The API request fails
/// - The response handling fails
pub async fn delete_secret(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    secret_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    // Clone the internal address from the endpoint configuration
    let address = omamori_endpoint.internal_address.clone();
    // Prepare the HTTP client based on the address and security requirements
    let client = prepare_client(&address, insecure_client);
    // Construct the URL for the secret deletion endpoint
    let url = format!("{address}/v1alpha/secret/{secret_uuid}");

    // Make the DELETE request to remove the secret
    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    // Handle the empty response and return the result
    handle_empty_response(response, "secret", &secret_uuid.to_string()).await
}
