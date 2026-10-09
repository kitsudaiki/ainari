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

//! Clients of the sakura-endpoints for the cold migration of a virtual_machine.
//!
//! The steps, which change something, are processed as tasks on the sakura-host, so they answer
//! with the task, which can be followed with `task::get_task`.

use std::io::Write;

use awc::http::StatusCode;
use futures_util::StreamExt;
use serde::Serialize;
use uuid::Uuid;

use ainari_api_structs::migration_structs::*;
use ainari_api_structs::task_structs::TaskResp;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

use crate::handle_response;
use crate::prepare_client;

/// Asks the source sakura-host to export a virtual_machine.
///
/// The virtual_machine is shut down and frozen in the state `MIGRATING`, so it can not be
/// started on the source anymore, while its files are pulled by the target.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the source sakura-host.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the virtual_machine to export.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the task of the export, or an `AinariError` on failure.
pub async fn export_virtual_machine(
    sakura_address: &str,
    token: &str,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let url = format!(
        "{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/migration/export/internal"
    );
    post_task(url, token, internal_api_key, None::<&()>, insecure_client).await
}

/// Asks the source sakura-host to unfreeze a virtual_machine, whose migration failed.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the source sakura-host.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the exported virtual_machine.
/// * `boot` - Boot the virtual_machine again, because it was running before the migration.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the task of the cancellation, or an `AinariError` on failure.
pub async fn cancel_export(
    sakura_address: &str,
    token: &str,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    boot: bool,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let url = format!(
        "{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/migration/cancel/internal"
    );
    let body = MigrationCancelReq { boot };
    post_task(url, token, internal_api_key, Some(&body), insecure_client).await
}

/// Asks the target sakura-host to take over a virtual_machine, which was exported by the source.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the target sakura-host.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `req` - The virtual_machine, its source and if it is booted afterwards.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the task of the import, or an `AinariError` on failure.
pub async fn import_virtual_machine(
    sakura_address: &str,
    token: &str,
    internal_api_key: &Secret,
    req: &MigrationImportReq,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let url = format!("{sakura_address}/v1alpha/virtual_machine/migration/internal");
    post_task(url, token, internal_api_key, Some(req), insecure_client).await
}

/// Removes the copy of a migrated virtual_machine from a sakura-host.
///
/// This is called for the source after a successful migration and for the target after a failed
/// one. Unlike a normal deletion, no trace of the virtual_machine is kept on the host, because
/// the virtual_machine itself still exists on the other host.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the sakura-host.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the virtual_machine.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the task of the removal, or an `AinariError` on failure.
pub async fn remove_migrated_virtual_machine(
    sakura_address: &str,
    token: &str,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url =
        format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/migration/internal");

    let response = client
        .delete(url)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(response, "migration", &virtual_machine_uuid.to_string()).await
}

/// Reads the description of an exported virtual_machine from the source sakura-host.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the source sakura-host.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the exported virtual_machine.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the description, or an `AinariError` on failure.
pub async fn get_migration_description(
    sakura_address: &str,
    token: &str,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    insecure_client: bool,
) -> Result<MigrationDescriptionResp, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url =
        format!("{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/migration/internal");

    let response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await;

    handle_response(response, "migration", &virtual_machine_uuid.to_string()).await
}

/// Streams a file of an exported virtual_machine from the source sakura-host into a sink.
///
/// The body is written into the sink as it is received, so files of any size can be transferred
/// without holding them in memory. The encoding of the body is up to the caller.
///
/// # Arguments
///
/// * `sakura_address` - The base URL of the source sakura-host.
/// * `token` - Authentication token for the API.
/// * `internal_api_key` - Internal API key for authorization.
/// * `virtual_machine_uuid` - UUID of the exported virtual_machine.
/// * `file` - The file to transfer.
/// * `sink` - Receives the body of the response.
/// * `insecure_client` - Whether to use an insecure client (no TLS verification).
///
/// # Returns
///
/// A `Result` containing the size of the original file in bytes, which the source announced in
/// the header `MIGRATION_FILE_SIZE_HEADER`, or an `AinariError` on failure.
pub async fn download_migration_file<W: Write>(
    sakura_address: &str,
    token: &str,
    internal_api_key: &Secret,
    virtual_machine_uuid: &Uuid,
    file: MigrationFile,
    sink: &mut W,
    insecure_client: bool,
) -> Result<u64, AinariError> {
    let client = prepare_client(sakura_address, insecure_client);
    let url = format!(
        "{sakura_address}/v1alpha/virtual_machine/{virtual_machine_uuid}/migration/file/{file}/internal"
    );

    // the timeout of the client only covers the time until the header arrived, the body is
    // streamed as long as it takes
    let mut response = client
        .get(url)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()))
        .send()
        .await
        .map_err(|e| {
            AinariError::InternalError(format!(
                "Failed to request {file} of virtual_machine '{virtual_machine_uuid}': {e}"
            ))
        })?;

    if response.status() != StatusCode::OK {
        let body = response.body().await.unwrap_or_default();
        let msg = format!(
            "Failed to download {file} of virtual_machine '{virtual_machine_uuid}': {} {}",
            response.status(),
            String::from_utf8_lossy(&body)
        );
        return Err(match response.status() {
            StatusCode::UNAUTHORIZED => AinariError::Unauthorized(msg),
            StatusCode::NOT_FOUND => AinariError::NotFound(msg),
            StatusCode::CONFLICT => AinariError::Conflict(msg),
            _ => AinariError::InternalError(msg),
        });
    }

    let file_size = response
        .headers()
        .get(MIGRATION_FILE_SIZE_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            AinariError::InternalError(format!(
                "Response for {file} of virtual_machine '{virtual_machine_uuid}' has no valid \
                 header '{MIGRATION_FILE_SIZE_HEADER}'"
            ))
        })?;

    while let Some(chunk) = response.next().await {
        let chunk = chunk.map_err(|e| {
            AinariError::InternalError(format!(
                "Transfer of {file} of virtual_machine '{virtual_machine_uuid}' broke off: {e}"
            ))
        })?;
        sink.write_all(&chunk).map_err(|e| {
            AinariError::InternalError(format!(
                "Failed to write {file} of virtual_machine '{virtual_machine_uuid}': {e}"
            ))
        })?;
    }

    Ok(file_size)
}

/// Sends a POST-request with an optional json-body to an endpoint, which answers with a task.
async fn post_task<T: Serialize>(
    url: String,
    token: &str,
    internal_api_key: &Secret,
    body: Option<&T>,
    insecure_client: bool,
) -> Result<TaskResp, AinariError> {
    let client = prepare_client(&url, insecure_client);
    let request = client
        .post(url)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .insert_header(("X-Internal-API-Key", internal_api_key.reveal()));

    let response = match body {
        Some(body) => {
            let json_str = serde_json::to_string(body).map_err(|e| {
                AinariError::InternalError(format!("Failed to serialize request: {e}"))
            })?;
            request
                .insert_header(("Content-Type", "application/json"))
                .send_body(json_str)
                .await
        }
        None => request.send().await,
    };

    handle_response(response, "migration-task", "").await
}
