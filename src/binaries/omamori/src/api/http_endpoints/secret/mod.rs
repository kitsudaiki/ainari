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

pub mod clone_secret_internal_v1_0;
pub mod delete_secret_internal_v1_0;
pub mod delete_secret_v1_0;
pub mod generate_secret_internal_v1_0;
pub mod generate_secret_v1_0;
pub mod get_secret_count_v1_0;
pub mod get_secret_payload_v1_0;
pub mod get_secret_v1_0;
pub mod list_secret_v1_0;
pub mod upload_secret_v1_0;

use crate::config;
use crate::database::secret_table;

use ainari_api::error_mapping::map_ainari_error_to_api_response;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::secret_structs::SECRET_OWNER_BACKUP;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;

/// Checks the quota for a new secret, which is owned by a resource of the given kind.
///
/// The secrets of backups don't count to the quota of the project, so they are always allowed.
///
/// # Arguments
///
/// * `owned_by` - The kind of owner of the new secret
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(())` - The secret can be created.
/// * `Err(ErrorResponse)` - The quota is exhausted or could not be checked.
async fn check_quota_for_owner(owned_by: &str, context: &UserContext) -> Result<(), ErrorResponse> {
    if owned_by == SECRET_OWNER_BACKUP {
        return Ok(());
    }
    check_quota(context).await
}

/// Checks if the user is still allowed to create another secret.
///
/// The current number of secrets of the whole project is counted in the database and compared
/// against the maximum, which is defined by the quota of the project in the miko.
///
/// # Arguments
///
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(())` - The project is still below the limit.
/// * `Err(ErrorResponse::Conflict)` - The quota of the project is already exhausted.
/// * `Err(ErrorResponse)` - The secrets could not be counted or the quota not be read.
async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of secrets of the whole project
    let current_number_of_secrets = secret_table::count_secrets_of_project(&context.project_id)
        .inspect_err(|e| {
            ainari_api::report_internal_error!(
                Some(context),
                "Failed to count secrets in database.: {e}"
            );
        })
        .map_err(|_| ErrorResponse::InternalError("Internal Error".into()))?;
    {};

    // check the maximum number of secrets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.project_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;

    // check if quota is already exceeded
    if current_number_of_secrets as i64 >= quota.max_secret as i64 {
        return Err(ErrorResponse::Conflict(
            "Maximum number of secrets exceeded.".to_string(),
        ));
    }

    Ok(())
}
