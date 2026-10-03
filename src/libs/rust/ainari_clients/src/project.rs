// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

use ainari_api_structs::project_structs::*;
use ainari_common::config as ainari_config;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Counts the resources of a project, which still exist within a component.
///
/// This function communicates with the internal endpoint of the component, which is used by miko
/// to check, if a project is empty, before it is deleted.
///
/// # Arguments
///
/// * `endpoint` - The endpoint configuration of the component (hanami, ryokan or omamori)
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `project_id` - The ID of the project, whose resources are counted
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the `ProjectResourceCountInternalResp` if successful, or an `AinariError`
/// if the operation fails.
pub async fn get_project_resource_count(
    endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    insecure_client: bool,
) -> Result<ProjectResourceCountInternalResp, AinariError> {
    let address = endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/resource_count/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(response, "project", project_id).await
}
