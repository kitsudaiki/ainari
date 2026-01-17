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

use ainari_api_structs::model_structs::*;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::prepare_client;
use crate::{handle_empty_response, handle_response};

/// Creates a new model in the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `name` - Name of the model to be created.
/// * `template` - Template to be used for model creation.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the created `ModelResp` on success, or an `AinariError` on failure.
pub async fn create_model(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    name: &str,
    template: &str,
    insecure_client: bool,
) -> Result<ModelResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/model/internal");

    // Create the request body with the provided name and template
    let body = ModelCreateReq {
        template: template.to_owned(),
        name: name.to_owned(),
    };
    let json_str = serde_json::to_string(&body).unwrap();

    // Send the POST request to create the model
    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    // Handle the response and return the result
    let resp: Result<ModelResp, AinariError> = handle_response(response, "model", "").await;
    resp
}

/// Retrieves information about a specific model from the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `model_uuid` - UUID of the model to retrieve.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the retrieved `ModelResp` on success, or an `AinariError` on failure.
pub async fn get_model(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    model_uuid: &Uuid,
    insecure_client: bool,
) -> Result<ModelResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/model/{model_uuid}/internal");

    // Send the GET request to retrieve the model information
    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    // Handle the response and return the result
    let resp: Result<ModelResp, AinariError> =
        handle_response(response, "model", &model_uuid.to_string()).await;
    resp
}

/// Lists all models available in the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the list of models as `ModelListResp` on success, or an `AinariError` on failure.
pub async fn list_model(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    insecure_client: bool,
) -> Result<ModelListResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/model/internal");

    // Send the GET request to list all models
    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    // Handle the response and return the result
    let resp: Result<ModelListResp, AinariError> = handle_response(response, "model", "").await;
    resp
}

/// Deletes a specific model from the Ainari system.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the Ainari Sakura service.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `model_uuid` - UUID of the model to delete.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` indicating success or failure. On success, returns `Ok(())`.
pub async fn delete_model(
    sakura_address: &String,
    token: &String,
    internal_api_key: &Secret,
    model_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!("{sakura_address}/v1alpha/model/{model_uuid}/internal");

    // Send the DELETE request to remove the model
    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    // Handle the empty response and return the result
    handle_empty_response(response, "model", &model_uuid.to_string()).await
}
