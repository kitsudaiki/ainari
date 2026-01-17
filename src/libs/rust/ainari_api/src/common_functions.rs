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

use tokio::fs;
use uuid::Uuid;

use crate::errors::ErrorResponse;

use ainari_api_structs::user_context::UserContext;
use ainari_clients::onsen_file_transfer;
use ainari_common::enums;
use ainari_common::error::AinariError;

/// Creates a directory and all necessary parent directories asynchronously.
///
/// # Arguments
///
/// * `path` - A string slice that holds the path to the directory to be created.
///
/// # Returns
///
/// * `Ok(())` if the directory was created successfully.
/// * `Err(ErrorResponse::InternalError)` if an error occurred during directory creation.
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

/// Uploads a file to an Onsen file transfer service asynchronously.
///
/// # Arguments
///
/// * `onsen_address` - A string slice that holds the address of the Onsen service.
/// * `remote_file_path` - A string slice that holds the destination path on the Onsen service.
/// * `local_file_path` - A string slice that holds the path to the local file to be uploaded.
///
/// # Returns
///
/// * `Ok(())` if the file was uploaded successfully.
/// * `Err(ErrorResponse::InternalError)` if an error occurred during the upload process.
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

/// Deletes a file from an Onsen file transfer service asynchronously.
///
/// # Arguments
///
/// * `onsen_address` - A string slice that holds the address of the Onsen service.
/// * `remote_file_path` - A string slice that holds the path to the file on the Onsen service to be deleted.
///
/// # Returns
///
/// * `Ok(())` if the file was deleted successfully.
/// * `Err(ErrorResponse::InternalError)` if an error occurred during the deletion process.
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

/// Verifies if the provided user context has admin privileges.
///
/// # Arguments
///
/// * `context` - A reference to a UserContext object containing user information.
///
/// # Returns
///
/// * `Ok(())` if the user has admin privileges.
/// * `Err(ErrorResponse::Unauthorized)` if the user does not have admin privileges.
pub fn check_admin_context(context: &UserContext) -> Result<(), ErrorResponse> {
    if context.is_admin != true.to_string() {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }

    Ok(())
}

/// Converts a string representation of a UUID into a Uuid object.
///
/// # Arguments
///
/// * `uuid` - A string slice that holds the UUID in string format.
///
/// # Returns
///
/// * `Ok(Uuid)` if the conversion was successful.
/// * `Err(ErrorResponse::InternalError)` if an error occurred during the conversion.
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

/// Maps AinariError to the appropriate ErrorResponse type.
///
/// This function translates internal AinariError types into the appropriate API error responses.
///
/// # Arguments
///
/// * `e` - An AinariError object to be mapped to an ErrorResponse.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the input AinariError.
pub fn map_ainari_error_to_api_response(e: AinariError) -> ErrorResponse {
    match e {
        AinariError::Unauthorized(msg) => ErrorResponse::Unauthorized(msg),
        AinariError::InvalidInput(msg) => ErrorResponse::BadRequest(msg),
        AinariError::InternalError(msg) => {
            log::error!("{msg}");
            ErrorResponse::InternalError("Internal Error".to_string())
        }
    }
}

/// Maps database errors for get and delete operations using ID to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being accessed.
/// * `id` - A string slice containing the ID of the object.
/// * `err` - A DbError enum indicating the type of database error.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_id_get_delete_error(obj_type: &str, id: &str, err: enums::DbError) -> ErrorResponse {
    match err {
        enums::DbError::InternalError => {
            log::error!("Error while deleting {obj_type} with ID '{id}' from DB");
            ErrorResponse::InternalError("Internal Error".to_string())
        }
        enums::DbError::NotFound => {
            ErrorResponse::NotFound(format!("{obj_type} with ID '{id}' not found."))
        }
    }
}

/// Maps database errors for get and delete operations using UUID to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being accessed.
/// * `uuid` - A reference to a Uuid object.
/// * `err` - A DbError enum indicating the type of database error.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_uuid_get_delete_error(
    obj_type: &str,
    uuid: &Uuid,
    err: enums::DbError,
) -> ErrorResponse {
    match err {
        enums::DbError::InternalError => {
            log::error!("Error while deleting {obj_type} with UUID '{uuid}' from DB");
            ErrorResponse::InternalError("Internal Error".to_string())
        }
        enums::DbError::NotFound => {
            ErrorResponse::NotFound(format!("{obj_type} with UUID '{uuid}' not found."))
        }
    }
}

/// Maps database errors for list operations to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being listed.
/// * `e` - A diesel::result::Error object indicating the database error.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_list_error(obj_type: &str, e: diesel::result::Error) -> ErrorResponse {
    log::error!("Failed to list {obj_type} with error: '{e}'");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Maps database errors for count operations to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being counted.
/// * `e` - A diesel::result::Error object indicating the database error.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_count_error(obj_type: &str, e: diesel::result::Error) -> ErrorResponse {
    log::error!("Failed to count {obj_type} with error: '{e}'");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Checks if an object with the given ID already exists in the database.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being checked.
/// * `id` - A string slice containing the ID of the object.
/// * `ret` - A Result object containing either the object or a DbError.
///
/// # Returns
///
/// * `Ok(())` if the object does not exist in the database.
/// * `Err(ErrorResponse::Conflict)` if the object already exists.
/// * `Err(ErrorResponse::InternalError)` if an internal database error occurred.
pub fn check_if_id_exist_in_db<T>(
    obj_type: &str,
    id: &str,
    ret: Result<T, enums::DbError>,
) -> Result<(), ErrorResponse> {
    match ret {
        Ok(_) => {
            let msg = format!("{obj_type} with ID '{id}' already exist.");
            return Err(ErrorResponse::Conflict(msg));
        }
        Err(enums::DbError::InternalError) => {
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            // it is desired, that the object not already exist, so this error will be ignored
        }
    };

    Ok(())
}
