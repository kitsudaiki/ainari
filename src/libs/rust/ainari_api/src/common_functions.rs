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

use ainari_common::config::MikoEndpoint;
use tokio::fs;
use uuid::Uuid;

use crate::errors::ErrorResponse;

use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::error::AinariError;

/// Creates a directory at the specified path.
///
/// # Arguments
///
/// * `path` - A string slice containing the path to the directory to be created
///
/// # Returns
///
/// * `Ok(())` - If the directory was created successfully
///
/// * `Err(ErrorResponse::InternalError)` - If there was an error creating the directory
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - Permission issues when trying to create directories
/// - Invalid path specifications
/// - Filesystem errors during directory creation
///
pub async fn create_directory(path: &String) -> Result<(), ErrorResponse> {
    match fs::create_dir_all(&path).await {
        Ok(_) => (),
        Err(e) => {
            log::error!("Failed to directory '{path}' with error: {e}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    }

    Ok(())
}

/// Uploads a file to an Onsen storage service.
///
/// # Arguments
///
/// * `onsen_address` - A string slice containing the address of the Onsen service
/// * `remote_file_path` - A string slice containing the destination path on the Onsen service
/// * `local_file_path` - A string slice containing the path to the local file to be uploaded
///
/// # Returns
///
/// * `Ok(())` - If the file was uploaded successfully
///
/// * `Err(ErrorResponse::InternalError)` - If there was an error during the upload process
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - Network errors when connecting to the Onsen service
/// - Authentication failures with the Onsen service
/// - Filesystem errors when accessing the local file
/// - Invalid path specifications for either local or remote paths
///
pub async fn upload_file_to_onsen(
    onsen_address: &str,
    remote_file_path: &str,
    local_file_path: &str,
) -> Result<(), ErrorResponse> {
    match onsen_file_transfer::upload_file(onsen_address, remote_file_path, local_file_path).await {
        Ok(()) => {}
        Err(e) => {
            log::error!(
                "Failed to send file with path '{local_file_path}' to onsen '{onsen_address}' to '{remote_file_path}' with error: {e}"
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    }

    Ok(())
}

/// Deletes a file from an Onsen storage service.
///
/// # Arguments
///
/// * `onsen_address` - A string slice containing the address of the Onsen service
/// * `remote_file_path` - A string slice containing the path of the file to be deleted on the Onsen service
///
/// # Returns
///
/// * `Ok(())` - If the file was deleted successfully
///
/// * `Err(ErrorResponse::InternalError)` - If there was an error during the deletion process
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - Network errors when connecting to the Onsen service
/// - Authentication failures with the Onsen service
/// - Invalid path specifications for the remote path
///
pub async fn delete_file_from_onsen(
    onsen_address: &str,
    remote_file_path: &str,
) -> Result<(), ErrorResponse> {
    match onsen_file_transfer::delete_file(onsen_address, remote_file_path).await {
        Ok(_) => {}
        Err(_) => {
            log::error!("Failed to delete file '{remote_file_path}' from onsen '{onsen_address}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    }

    Ok(())
}

/// Checks if the provided user context has admin privileges.
///
/// # Arguments
///
/// * `context` - A reference to a `UserContext` containing the user's information
///
/// # Returns
///
/// * `Ok(())` - If the user has admin privileges
///
/// * `Err(ErrorResponse::Unauthorized)` - If the user does not have admin privileges
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - When the provided context does not have admin privileges
///
pub fn check_admin_context(context: &UserContext) -> Result<(), ErrorResponse> {
    if !context.is_admin {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }

    Ok(())
}

/// Converts a string representation of a UUID into a proper UUID type.
///
/// # Arguments
///
/// * `uuid` - A string slice containing the UUID in string format
///
/// # Returns
///
/// * `Ok(Uuid)` - If the string was successfully converted to a UUID
///
/// * `Err(ErrorResponse::InternalError)` - If there was an error during the conversion process
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - Invalid UUID string format
/// - Parsing errors when converting the string to UUID
///
pub fn convert_uuid(uuid: &String) -> Result<Uuid, ErrorResponse> {
    let uuid = match Uuid::parse_str(uuid) {
        Ok(uuid) => uuid,
        Err(e) => {
            log::error!("Failed to convert string '{uuid}' into uuid with error: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(uuid)
}

pub async fn get_endpoints(
    miko_endpoint: &MikoEndpoint,
    insecure_connection: bool,
) -> Result<ainari_common::config::Endpoints, ErrorResponse> {
    let endpoints =
        match ainari_clients::endpoints::get_endpoints(miko_endpoint, insecure_connection).await {
            Ok(body) => body,
            Err(AinariError::Unauthorized(msg)) => {
                return Err(ErrorResponse::Unauthorized(msg));
            }
            Err(AinariError::InvalidInput(msg)) => {
                return Err(ErrorResponse::BadRequest(msg));
            }
            Err(AinariError::Error(msg)) => {
                log::error!("{msg}");
                return Err(ErrorResponse::InternalError("Internal Error".to_string()));
            }
        };

    Ok(endpoints)
}
