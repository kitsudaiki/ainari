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

use ainari_api_structs::exports::{
    HanamiProjectExport, MikoProjectExport, OmamoriProjectExport, OmamoriProjectExportReq,
    RyokanProjectExport, SakuraVirtualMachineExport,
};
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Exports a project together with its quota from miko for a backup.
///
/// # Arguments
///
/// * `miko_endpoint` - The endpoint of miko, whose internal address is used
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `project_id` - The ID of the project to export
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the project with its quota, or an `AinariError`.
pub async fn export_project(
    miko_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    insecure_client: bool,
) -> Result<MikoProjectExport, AinariError> {
    let address = miko_endpoint.internal_address.clone();
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

/// Exports the images and snapshots of a project from ryokan for a backup.
///
/// # Arguments
///
/// * `ryokan_endpoint` - The endpoint configuration for the Ryokan service
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `project_id` - The ID of the project to export
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the metadata of the images, or an `AinariError`.
pub async fn export_project_images(
    ryokan_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    insecure_client: bool,
) -> Result<RyokanProjectExport, AinariError> {
    let address = ryokan_endpoint.internal_address.clone();
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

/// Exports the secrets and public-keys of a project from the omamori for a backup.
///
/// The payloads of the secrets are wrapped with the wrap-key, so they never leave the omamori in
/// plain text.
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the omamori service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `project_id` - The ID of the project to export
/// * `wrap_key` - The base64-encoded key with 256 bit, which wraps the payloads
/// * `backup_uuid` - The backup, whose own secrets are exported too
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the secrets and public-keys of the project, or an `AinariError`.
pub async fn export_project_secrets(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    wrap_key: &Secret,
    backup_uuid: Option<&Uuid>,
    insecure_client: bool,
) -> Result<OmamoriProjectExport, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/export/internal");

    let body = OmamoriProjectExportReq {
        wrap_key: wrap_key.clone(),
        backup_uuid: backup_uuid.copied(),
    };

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response(response, "project", project_id).await
}

/// Exports the config of a virtual_machine from its sakura-host for a backup.
///
/// # Arguments
///
/// * `sakura_address` - Internal address of the sakura-host, which runs the virtual_machine
/// * `token` - The authentication token for the API
/// * `internal_api_key` - The internal API key for accessing protected endpoints
/// * `virtual_machine_uuid` - The virtual_machine to export
/// * `insecure_client` - Whether to use an insecure (HTTP) client instead of HTTPS
///
/// # Returns
///
/// A `Result` containing the config of the virtual_machine, or an `AinariError`.
pub async fn export_virtual_machine(
    sakura_address: &str,
    token: &String,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<SakuraVirtualMachineExport, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url =
        format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/export/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(
        response,
        "virtual_machine",
        &virtual_machine_uuid.to_string(),
    )
    .await
}
