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

pub mod create_image_v1_0;
pub mod delete_image_v1_0;
pub mod get_image_count_v1_0;
pub mod get_image_internal_v1_0;
pub mod get_image_v1_0;
pub mod init_image_internal_v1_0;
pub mod list_image_v1_0;
pub mod update_image_internal_v1_0;

use crate::config;
use crate::database::image_table::{self, ImageEntry};

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::image_structs::ImageStatus;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::quota::get_quota;
use ainari_clients::secret::delete_secret;
use uuid::Uuid;

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

/// Returns the status of an image from the database as value of the api.
///
/// # Arguments
///
/// * `image_data` - The image from the database
///
/// # Returns
///
/// * `Ok(ImageStatus)` - The status of the image
/// * `Err(ErrorResponse::InternalError)` - If the database contains an unknown status
fn image_status_of(image_data: &ImageEntry) -> Result<ImageStatus, ErrorResponse> {
    image_data.image_status.parse().map_err(|e| {
        ainari_api::report_internal_error!(
            None,
            "Invalid status of image '{}' in database: {e}",
            image_data.uuid
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })
}

/// Checks if the project has reached its image quota limit.
///
/// # Arguments
///
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(())` - If the project is within its image quota limit
///
/// * `Err(ErrorResponse::Unauthorized)` - If the user is not authorized to check their quota
/// * `Err(ErrorResponse::BadRequest)` - If the input to the quota check is invalid
/// * `Err(ErrorResponse::Conflict)` - If the project has exceeded its image quota
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error checking the quota
///
async fn check_image_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of images of the whole project
    let current_number_of_images = image_table::count_images_of_project(&context.project_id)
        .map_err(|e| {
            ainari_api::report_internal_error!(
                Some(context),
                "Failed to count images in database.: {e}"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    // check the maximum number of images defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.project_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;

    let max_number_of_images = quota.max_image as i64;
    // check if quota is already exceeded
    if current_number_of_images as i64 >= max_number_of_images {
        return Err(ErrorResponse::Conflict(
            "Maximum number of images exceeded.".to_string(),
        ));
    }

    Ok(())
}

/// Removes an image, whose upload failed, together with its file in the onsen and its secret.
///
/// This is used for cleanup, so failures are only logged and not reported back: the upload
/// already has its own error, which must not be masked by an error of the cleanup itself. Parts,
/// which don't exist yet, like the file in the onsen, are skipped.
///
/// # Arguments
///
/// * `image_uuid` - UUID of the failed image
/// * `onsen_address` - Address of the onsen, where the file of the image is stored
/// * `file_path` - Path of the file of the image in the onsen
/// * `secret_uuid` - UUID of the secret of the image
/// * `context` - The user context of the upload
async fn remove_failed_image(
    image_uuid: &Uuid,
    onsen_address: &str,
    file_path: &str,
    secret_uuid: &Uuid,
    context: &UserContext,
) {
    // the image is not in the database, if the upload failed before it was registered
    match image_table::delete_image(image_uuid, context) {
        Ok(_) | Err(ainari_common::enums::DbError::NotFound) => {}
        Err(_) => log::error!("Failed to delete failed image {image_uuid} from database."),
    }

    // onsen accepts the deletion of a file, which doesn't exist
    let _ = delete_file_from_onsen(onsen_address, file_path, Some(context)).await;

    let endpoints = match get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
    {
        Ok(endpoints) => endpoints,
        Err(e) => {
            log::error!("Failed to delete secret {secret_uuid} of failed image {image_uuid}: {e}");
            return;
        }
    };
    if let Err(e) = delete_secret(
        &endpoints.omamori,
        &context.token,
        &config::INTERNAL_API_KEY,
        secret_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    {
        log::error!("Failed to delete secret {secret_uuid} of failed image {image_uuid}: {e}");
    }
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
