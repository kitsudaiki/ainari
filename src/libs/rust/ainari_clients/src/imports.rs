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

use ainari_api_structs::exports::MikoProjectExport;
use ainari_api_structs::imports::{
    HanamiProjectImportReq, HanamiProjectImportResp, MikoProjectImportReq, MikoProjectImportResp,
    OmamoriProjectImportReq, OmamoriProjectImportResp, RyokanProjectImportReq,
    RyokanProjectImportResp,
};
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Imports a project together with its quota from a backup into miko.
///
/// # Arguments
///
/// * `miko_endpoint` - The endpoint of miko, whose internal address is used
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `project_id` - The ID, under which the project is imported
/// * `project` - The project from the backup
/// * `force` - Overwrite an existing project
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the result of the import, or an `AinariError`. An existing project
/// without `force` is rejected with `Conflict`.
pub async fn import_project(
    miko_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    project: &MikoProjectExport,
    force: bool,
    insecure_client: bool,
) -> Result<MikoProjectImportResp, AinariError> {
    let address = miko_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/import/internal");

    let body = MikoProjectImportReq {
        project: project.clone(),
        force,
    };

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response(response, "project", project_id).await
}

/// Imports the images and snapshots of a project from a backup into ryokan, whose files were
/// already copied to normal onsens.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `project_id` - The ID of the project, into which is imported
/// * `request` - The images with the locations of their files, `force` and the `id_map`
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the imported images with their UUIDs, or an `AinariError`. Conflicts
/// with existing images are rejected with `Conflict`.
pub async fn import_project_images(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    request: &RyokanProjectImportReq,
    insecure_client: bool,
) -> Result<RyokanProjectImportResp, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/import/internal");

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(request)
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

/// Imports the secrets and public-keys of a project from a backup into the omamori.
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the omamori service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `project_id` - The ID of the project, into which is imported
/// * `request` - The secrets and public-keys from the backup together with the wrap-key,
///   `force` and the `id_map`
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the imported resources with their UUIDs, or an `AinariError`. Conflicts
/// with existing resources are rejected with `Conflict`, a wrong wrap-key with `InvalidInput`.
pub async fn import_project_secrets(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    request: &OmamoriProjectImportReq,
    insecure_client: bool,
) -> Result<OmamoriProjectImportResp, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/import/internal");

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(request)
        .await;

    handle_response(response, "project", project_id).await
}
