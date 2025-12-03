// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod check_dataset_v1_0;
pub mod create_dataset_v1_0;
pub mod delete_dataset_v1_0;
pub mod get_dataset_internal_v1_0;
pub mod get_dataset_v1_0;
pub mod init_dataset_internal_v1_0;
pub mod list_dataset_v1_0;

use crate::config;
use crate::database::dataset_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;

/// Validates that the provided dataset type is one of the supported types.
///
/// This function checks if the given `dataset_type` is either "mnist" or "csv".
/// If the type is not in the supported list, it returns an `ErrorResponse::BadRequest`
/// with a descriptive message. Otherwise, it returns `Ok(())` indicating the type is valid.
///
/// # Arguments
///
/// * `dataset_type` - A string slice containing the dataset type to validate.
///
/// # Returns
///
/// * `Ok(())` - If the dataset type is valid ("mnist" or "csv").
/// * `Err(ErrorResponse::BadRequest)` - If the dataset type is not in the supported list.
///
fn check_dataset_type(dataset_type: &String) -> Result<(), ErrorResponse> {
    if !["mnist", "csv"].contains(&dataset_type.as_str()) {
        let msg = format!("Type '{dataset_type}' is not in list [ mnist, csv ]");
        return Err(ErrorResponse::BadRequest(msg.to_string()));
    }

    Ok(())
}

/// Checks if the user has reached their dataset quota limit.
///
/// # Arguments
///
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(())` - If the user is within their dataset quota limit
///
/// * `Err(ErrorResponse::Unauthorized)` - If the user is not authorized to check their quota
/// * `Err(ErrorResponse::BadRequest)` - If the input to the quota check is invalid
/// * `Err(ErrorResponse::Conflict)` - If the user has exceeded their dataset quota
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error checking the quota
///
async fn check_dataset_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of datasets of the user
    let current_number_of_datasets = dataset_table::count_datasets(context).map_err(|e| {
        log::error!("Failed to count datasets in database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // check the maximum number of datasets defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    let max_number_of_datasets = quota.max_dataset as i64;
    // check if quota is already exceeded
    if current_number_of_datasets as i64 >= max_number_of_datasets {
        return Err(ErrorResponse::Conflict(
            "Maximum number of datasets exceeded.".to_string(),
        ));
    }

    Ok(())
}

fn remove_all(target_dir_path: &String) {
    // delete all temporary files
    let _ = std::fs::remove_dir_all(target_dir_path).map_err(|e| {
        log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
    });
}
