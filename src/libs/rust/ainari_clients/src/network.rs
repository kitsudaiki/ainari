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

use ainari_api_structs::network_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Exports the networks, virtual_machines, packet filters and floating ip-addresses of a project
/// from hanami for a backup.
///
/// # Arguments
///
/// * `hanami_endpoint` - The endpoint configuration for the hanami service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `project_id` - The ID of the project to export
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the resources of the project, or an `AinariError`.
pub async fn export_project_networks(
    hanami_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    insecure_client: bool,
) -> Result<HanamiProjectExport, AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/export/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(response, "project", project_id).await
}

/// Imports networks, packet filters and floating ip-addresses of a project from a backup into
/// hanami.
///
/// # Arguments
///
/// * `hanami_endpoint` - The endpoint configuration for the hanami service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `project_id` - The ID of the project, into which is imported
/// * `request` - The resources from the backup, `force` and the `id_map`
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the imported resources with their UUIDs, or an `AinariError`. Conflicts
/// with existing resources are rejected with `Conflict`.
pub async fn import_project_networks(
    hanami_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    request: &HanamiProjectImportReq,
    insecure_client: bool,
) -> Result<HanamiProjectImportResp, AinariError> {
    let address = hanami_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/import/internal");

    // the packet filters are applied on the torii one after another, which takes its time
    let response = client
        .post(url)
        .timeout(std::time::Duration::from_secs(10 * 60))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(request)
        .await;

    handle_response(response, "project", project_id).await
}
