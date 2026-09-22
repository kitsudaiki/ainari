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

pub mod check_image_v1_0;
pub mod create_image_v1_0;
pub mod delete_image_v1_0;
pub mod get_image_count_v1_0;
pub mod get_image_internal_v1_0;
pub mod get_image_v1_0;
pub mod init_image_internal_v1_0;
pub mod list_image_v1_0;

use crate::config;
use crate::database::image_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;

/// Validates that the provided image type is one of the supported types.
///
/// If the type is not in the supported list, it returns an `ErrorResponse::BadRequest`
/// with a descriptive message. Otherwise, it returns `Ok(())` indicating the type is valid.
///
/// # Arguments
///
/// * `image_type` - A string slice containing the image type to validate.
///
/// # Returns
///
/// * `Ok(())` - If the image type is valid ("disk").
/// * `Err(ErrorResponse::BadRequest)` - If the image type is not in the supported list.
///
fn check_image_type(image_type: &String) -> Result<(), ErrorResponse> {
    if !["disk"].contains(&image_type.as_str()) {
        let msg = format!("Type '{image_type}' is not in list [ disk ]");
        return Err(ErrorResponse::BadRequest(msg.to_string()));
    }

    Ok(())
}

/// Checks if the user has reached their image quota limit.
///
/// # Arguments
///
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(())` - If the user is within their image quota limit
///
/// * `Err(ErrorResponse::Unauthorized)` - If the user is not authorized to check their quota
/// * `Err(ErrorResponse::BadRequest)` - If the input to the quota check is invalid
/// * `Err(ErrorResponse::Conflict)` - If the user has exceeded their image quota
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error checking the quota
///
async fn check_image_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of images of the user
    let current_number_of_images = image_table::count_images(context).map_err(|e| {
        log::error!("Failed to count images in database.: {e}");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // check the maximum number of images defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    let max_number_of_images = quota.max_image as i64;
    // check if quota is already exceeded
    if current_number_of_images as i64 >= max_number_of_images {
        return Err(ErrorResponse::Conflict(
            "Maximum number of images exceeded.".to_string(),
        ));
    }

    Ok(())
}

/// Deletes the temporary directory of an upload together with its content.
///
/// This is used for cleanup, so a failure is only logged and not reported back: the operation,
/// which triggered the cleanup, already has its own result and must not be masked by an error of
/// the cleanup itself.
///
/// # Arguments
///
/// * `target_dir_path` - Path of the temporary directory to delete
fn remove_all(target_dir_path: &String) {
    // delete all temporary files
    let _ = std::fs::remove_dir_all(target_dir_path).map_err(|e| {
        log::error!("Failed to delete temp-dir {target_dir_path} from disk with error {e}.");
    });
}
