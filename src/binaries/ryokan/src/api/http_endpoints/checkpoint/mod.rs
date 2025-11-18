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

pub mod delete_checkpoint_v1_0;
pub mod get_checkpoint_internal_v1_0;
pub mod get_checkpoint_v1_0;
pub mod init_checkpoint_internal_v1_0;
pub mod list_checkpoint_v1_0;

use uuid::Uuid;

use crate::config;
use crate::database::checkpoint_table;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::checkpoint_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::quota::get_quota;
use ainari_common::enums;
use ainari_common::error::AinariError;

/// Checks if the user has reached their checkpoint quota limit.
///
/// # Arguments
///
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(())` - If the user is within their checkpoint quota limit
///
/// * `Err(ErrorResponse::Unauthorized)` - If the user is not authorized to check their quota
/// * `Err(ErrorResponse::BadRequest)` - If the input to the quota check is invalid
/// * `Err(ErrorResponse::Conflict)` - If the user has exceeded their checkpoint quota
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error checking the quota
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - Database access failures when counting checkpoints
/// - Network errors when communicating with the Miko service
/// - Authentication failures when accessing the Miko service
/// - Invalid input parameters when checking the quota
///
async fn check_checkpoint_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // get number of checkpoints of the user
    let current_number_of_checkpoints = match checkpoint_table::count_checkpoints(context) {
        Ok(number) => number,
        Err(e) => {
            log::error!("Failed to count checkpoints in database.: {e}");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // check the maximum number of checkpoints defined in miko
    let miko_endpoint = &config::CONFIG.miko;
    let max_number_of_checkpoints = match get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.insecure_clients,
    )
    .await
    {
        Ok(body) => body.max_checkpoint as i64,
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

    // check if quota is already exceeded
    if current_number_of_checkpoints as i64 >= max_number_of_checkpoints {
        return Err(ErrorResponse::Conflict(
            "Maximum number of checkpoints exceeded.".to_string(),
        ));
    }

    Ok(())
}
/// Adds a new checkpoint to the database.
///
/// # Arguments
///
/// * `checkpoint_uuid` - A reference to the UUID of the checkpoint to be added
/// * `checkpoint_name` - A reference to the name of the checkpoint
/// * `onsen_address` - A reference to the onsen address associated with the checkpoint
/// * `file_path` - A reference to the file path where the checkpoint is stored
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(())` - If the checkpoint was successfully added to the database
///
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error adding the checkpoint
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - Database access failures when adding the new checkpoint
/// - Invalid input parameters when creating the checkpoint record
///
fn add_checkpoint_to_database(
    checkpoint_uuid: &Uuid,
    checkpoint_name: &str,
    onsen_address: &str,
    file_path: &str,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    // add new checkpoint to datbase
    match checkpoint_table::add_new_checkpoint(
        checkpoint_uuid,
        checkpoint_name,
        onsen_address,
        file_path,
        context,
    ) {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("Failed to add checkpoint to database: {e}");
            log::error!("{msg}");
            return Err(ErrorResponse::InternalError(msg));
        }
    }

    Ok(())
}

/// Retrieves a checkpoint from the database based on its UUID.
///
/// # Arguments
///
/// * `checkpoint_uuid` - A reference to the UUID of the checkpoint to retrieve
/// * `context` - A reference to the `UserContext` containing user information and authentication token
///
/// # Returns
///
/// * `Ok(CheckpointInternalResp)` - A response object containing the checkpoint information if found
///
/// * `Err(ErrorResponse::InternalError)` - If there was an internal error accessing the database
/// * `Err(ErrorResponse::NotFound)` - If no checkpoint was found with the provided UUID
///
/// # Errors
///
/// This function will return errors in the following scenarios:
/// - Database access failures when retrieving the checkpoint
/// - Invalid UUID formats when querying the database
/// - Authentication failures when accessing the database
///
fn get_checkpoint_internal(
    checkpoint_uuid: &Uuid,
    context: &UserContext,
) -> Result<CheckpointInternalResp, ErrorResponse> {
    // get new created checkpoint from database to get addtional information
    match checkpoint_table::get_checkpoint(checkpoint_uuid, context) {
        Ok(checkpoint) => {
            let resp = CheckpointInternalResp {
                uuid: *checkpoint_uuid,
                name: checkpoint.name.clone(),
                onsen_address: checkpoint.onsen_address.clone(),
                file_path: checkpoint.file_path.clone(),
                created_by: checkpoint.created_by.clone(),
                created_at: checkpoint.created_at.clone(),
                updated_by: checkpoint.updated_by.clone(),
                updated_at: checkpoint.updated_at.clone(),
            };

            Ok(resp)
        }
        Err(enums::DbError::InternalError) => {
            log::error!("Error while getting checkpoint from DB");
            Err(ErrorResponse::InternalError("Internal Error".to_string()))
        }
        Err(enums::DbError::NotFound) => {
            let msg = format!("Checkpoint with UUID '{checkpoint_uuid}' not found.");
            Err(ErrorResponse::NotFound(msg))
        }
    }
}
