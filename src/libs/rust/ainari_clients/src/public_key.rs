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

use ainari_api_structs::public_key_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Retrieves an existing public-key together with the key itself.
///
/// This function fetches the details of a public-key identified by its UUID.
/// It communicates with the internal endpoint of the Omamori service, because only this
/// one returns the public-key itself instead of only its fingerprint.
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the Omamori service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `public_key_uuid` - The unique identifier of the public-key to retrieve
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the `PublicKeyInternalResp` if successful, or an `AinariError` if the operation fails.
pub async fn get_public_key(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    public_key_uuid: &Uuid,
    insecure_client: bool,
) -> Result<PublicKeyInternalResp, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/public_key/{public_key_uuid}/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    let resp: Result<PublicKeyInternalResp, AinariError> =
        handle_response(response, "public_key", &public_key_uuid.to_string()).await;
    resp
}
