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

use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_empty_response;
use crate::prepare_client;
use uuid::Uuid;

/// Deletes a network in hanami.
///
/// The internal API key is sent as well, so the resource can also be deleted, while the project is
/// locked by a restore, which prunes the project.
///
/// # Arguments
///
/// * `hanami_endpoint` - The endpoint configuration for the hanami service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key of the calling component
/// * `network_uuid` - The network to delete
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` indicating success or an `AinariError`.
pub async fn delete_network(
    hanami_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    network_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/network/{network_uuid}");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "network", &network_uuid.to_string()).await
}
