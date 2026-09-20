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

use ainari_api_structs::image_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Initializes a new image in the Ryokan service.
///
/// This function creates a new image with the specified parameters in the Ryokan service.
/// It requires proper authentication and authorization to access the internal API.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `image_uuid` - The unique identifier for the image
/// * `name` - The human-readable name for the image
/// * `dimension` - A tuple containing the number of rows and column names for the image
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the image response if successful, or an `AinariError` if the operation fails.
pub async fn init_image_in_ryokan(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    image_uuid: &Uuid,
    name: &str,
    dimension: (u64, Vec<String>),
    insecure_client: bool,
) -> Result<ImageInternalResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/image/internal");

    let body = ImageInitReq {
        uuid: *image_uuid,
        name: name.to_owned(),
        image_type: "csv".to_string(), // Note: Hardcoded image type - consider making this configurable
        number_of_rows: dimension.0,
        column_names: dimension.1,
    };

    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<ImageInternalResp, AinariError> = handle_response(response, "image", "").await;
    resp
}

/// Retrieves an existing image from the Ryokan service.
///
/// This function fetches information about a specific image identified by its UUID.
/// It requires proper authentication and authorization to access the internal API.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `image_uuid` - The unique identifier for the image to retrieve
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the image response if successful, or an `AinariError` if the operation fails.
pub async fn get_image(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    image_uuid: &Uuid,
    insecure_client: bool,
) -> Result<ImageInternalResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/image/{image_uuid}/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send()
        .await;

    // Handle the response and return the result
    handle_response::<ImageInternalResp>(response, "image", &image_uuid.to_string()).await
}
