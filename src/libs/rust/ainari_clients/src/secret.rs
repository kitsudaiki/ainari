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

use ainari_api_structs::project_structs::{
    OmamoriProjectExport, OmamoriProjectExportReq, OmamoriProjectImportReq,
    OmamoriProjectImportResp,
};
use ainari_api_structs::secret_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_empty_response;
use crate::handle_response;
use crate::prepare_client;

/// Generates a new secret with the given name
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the Omamori service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
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
    internal_api_key: &Secret,
    name: &str,
    owned_by: &str,
    resource_uuid: &Uuid,
    insecure_client: bool,
) -> Result<SecretResp, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/secret/generate/internal");

    let body = SecretGenerateInternalReq {
        name: name.to_owned(),
        owned_by: owned_by.to_owned(),
        resource_uuid: *resource_uuid,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<SecretResp, AinariError> = handle_response(response, "secret", "").await;
    resp
}

/// Clones an existing secret into a new secret with the same payload, owned by a resource
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the Omamori service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `secret_uuid` - The UUID of the secret to clone
/// * `owned_by` - The kind of owner of the new secret
/// * `resource_uuid` - The UUID of the resource, which owns the new secret
/// * `insecure_client` - Whether to create an insecure HTTP client (for testing purposes)
///
/// # Returns
///
/// A `Result` containing either the response of the new secret or an error
///
/// # Errors
///
/// This function will return an error if:
/// - The client preparation fails
/// - The API request fails
/// - The response handling fails
pub async fn clone_secret(
    omamori_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    secret_uuid: &Uuid,
    owned_by: &str,
    resource_uuid: &Uuid,
    insecure_client: bool,
) -> Result<SecretResp, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/secret/clone/internal");

    let body = SecretCloneInternalReq {
        secret_uuid: *secret_uuid,
        owned_by: owned_by.to_owned(),
        resource_uuid: *resource_uuid,
    };
    let json_str = serde_json::to_string(&body).unwrap();

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .insert_header(("Content-Type", "application/json"))
        .send_body(json_str)
        .await;

    let resp: Result<SecretResp, AinariError> =
        handle_response(response, "secret", &secret_uuid.to_string()).await;
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
) -> Result<Secret, AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/secret/{secret_uuid}/payload");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .send()
        .await;

    let resp: SecretWithPayloadResp =
        handle_response(response, "secret", &secret_uuid.to_string()).await?;

    Ok(Secret::from(resp.secret_payload))
}

/// Deletes an existing secret
///
/// # Arguments
///
/// * `omamori_endpoint` - The endpoint configuration for the Omamori service
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
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
    internal_api_key: &Secret,
    secret_uuid: &Uuid,
    insecure_client: bool,
) -> Result<(), AinariError> {
    let address = omamori_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/secret/{secret_uuid}/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_empty_response(response, "secret", &secret_uuid.to_string()).await
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
