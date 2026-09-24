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

pub mod create_virtual_machine_v1_0;
pub mod delete_virtual_machine_internal_v1_0;
pub mod get_virtual_machine_internal_v1_0;
pub mod list_virtual_machine_internal_v1_0;
pub mod reserve_virtual_machine_internal_v1_0;
pub mod snapshot_restore_v1_0;
pub mod snapshot_save_v1_0;

use uuid::Uuid;

use crate::config;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::secret::get_secret_payload;
use ainari_common::secret::Secret;

/// Retrieves a secret from the secret service.
///
/// This asynchronous function fetches secret payload information from the
/// secret service using the provided UUID and user context.
///
/// # Arguments
/// * `secret_uuid` - The UUID of the secret to retrieve
/// * `context` - The user context for authentication
///
/// # Returns
/// * `Result<Secret, ErrorResponse>` - The retrieved secret or an error
async fn get_secret(secret_uuid: &Uuid, context: &UserContext) -> Result<Secret, ErrorResponse> {
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    let secret_payload = get_secret_payload(
        &endpoints.omamori,
        &context.token,
        secret_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok(Secret::from(secret_payload.secret_payload))
}

/// Removes all files and directories in the specified target directory.
///
/// This function performs a complete cleanup of the specified directory,
/// removing all files and subdirectories within it.
///
/// # Arguments
/// * `target_dir_path` - The path to the directory to remove
#[allow(dead_code)]
fn remove_all(target_dir_path: &String) {
    // delete all temporary files
    let _ = std::fs::remove_dir_all(target_dir_path).map_err(|e| {
        log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
    });
}
