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

pub mod create_token_v1_0;
pub mod renew_token_v1_0;
pub mod validate_token_v1_0;

use crate::database::project_table;
use crate::database::user_project_mapping_table;

use ainari_api::errors::ErrorResponse;
use ainari_common::enums::{DbError, ProjectRole};

/// Gets the role of a user within a project, for which a new token is created.
///
/// A project, which doesn't exist, and a project, to which the user is not assigned, give the
/// same error, so the response doesn't reveal, which projects exist.
///
/// # Arguments
///
/// * `user_id` - ID of the user, for which the token is created
/// * `project_id` - ID of the project, for which the token is created
///
/// # Returns
///
/// * `Ok(ProjectRole)` - The role of the user within the project.
/// * `Err(ErrorResponse::Unauthorized)` - The user has no access to the project.
/// * `Err(ErrorResponse::InternalError)` - The database could not be read.
pub(crate) fn get_project_role_for_token(
    user_id: &String,
    project_id: &String,
) -> Result<ProjectRole, ErrorResponse> {
    let no_access_msg = format!("User has no access to project '{project_id}'");
    let map_error = |e| match e {
        DbError::NotFound | DbError::PermissionDenied => {
            ErrorResponse::Unauthorized(no_access_msg.clone())
        }
        DbError::InternalError => ErrorResponse::InternalError("Internal Error".to_string()),
    };

    // check if the project exist
    project_table::get_auth_project(project_id).map_err(map_error)?;

    // get the role of the user within the project
    let mapping =
        user_project_mapping_table::get_mapping(project_id, user_id).map_err(map_error)?;

    Ok(mapping.role)
}
