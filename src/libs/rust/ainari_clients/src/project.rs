// Copyright 2022-2026 Tobias Anker <tobias.anker@kitsunemimi.moe>

use uuid::Uuid;

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

/// Locks a project in miko for a job, like a backup or a restore.
///
/// While the project is locked, its users can only read its resources. Locking a project, which
/// is already locked by the same job, is accepted, so the call can be repeated.
///
/// # Arguments
///
/// * `miko_endpoint` - The endpoint of miko, whose internal address is used
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `project_id` - The ID of the project to lock
/// * `job_uuid` - The job, which locks the project
/// * `reason` - Description, why the project is locked
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the lock of the project, or an `AinariError`. A project, which is
/// already locked by another job, is rejected with `Conflict`.
pub async fn lock_project(
    miko_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    job_uuid: &Uuid,
    reason: &str,
    insecure_client: bool,
) -> Result<ProjectLockResp, AinariError> {
    let address = miko_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/lock/internal");

    let body = ProjectLockReq {
        job_uuid: *job_uuid,
        reason: reason.to_owned(),
    };

    let response = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response(response, "project", project_id).await
}

/// Removes the lock of a project in miko, which was set by the given job.
///
/// Unlocking a project, which is not locked, is accepted, so the call can be repeated.
///
/// # Arguments
///
/// * `miko_endpoint` - The endpoint of miko, whose internal address is used
/// * `token` - The authentication token for the API request
/// * `internal_api_key` - The internal API key for authentication
/// * `project_id` - The ID of the project to unlock
/// * `job_uuid` - The job, which has locked the project
/// * `insecure_client` - Whether to use an insecure (non-TLS) client
///
/// # Returns
///
/// A `Result` containing the lock of the project, which is None now, or an `AinariError`. A
/// project, which is locked by another job, is rejected with `Conflict`.
pub async fn unlock_project(
    miko_endpoint: &ainari_config::Endpoint,
    token: &String,
    internal_api_key: &Secret,
    project_id: &str,
    job_uuid: &Uuid,
    insecure_client: bool,
) -> Result<ProjectLockResp, AinariError> {
    let address = miko_endpoint.internal_address.clone();
    let client = prepare_client(&address, insecure_client);
    let url = format!("{address}/v1alpha/project/{project_id}/lock/internal");

    let body = ProjectUnlockReq {
        job_uuid: *job_uuid,
    };

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send_json(&body)
        .await;

    handle_response(response, "project", project_id).await
}

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
