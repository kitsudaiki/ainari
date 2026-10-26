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

pub mod change_passphrase_admin_v1_0;
pub mod change_passphrase_v1_0;
pub mod create_token_v1_0;
pub mod renew_token_for_project_admin_v1_0;
pub mod renew_token_v1_0;
pub mod validate_token_v1_0;

use crate::database::project_table;
use crate::database::user_project_mapping_table;
use crate::database::user_table::UserEntry;

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
        DbError::InternalError => {
            ainari_api::report_internal_error!(
                None,
                "Failed to get the role of user '{user_id}' in project '{project_id}' from DB"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        }
    };

    // check if the project exist
    project_table::get_auth_project(project_id).map_err(map_error)?;

    // get the role of the user within the project
    let mapping =
        user_project_mapping_table::get_mapping(project_id, user_id).map_err(map_error)?;

    Ok(mapping.role)
}

/// Gets the role of a global admin within a project, for which a new token is created over the
/// admin-endpoint.
///
/// Unlike `get_project_role_for_token`, the admin doesn't have to be assigned to the project. This
/// is only used by the separate admin-endpoint, so the normal creation and renewal of tokens never
/// gives a user access to a project, to which the user is not assigned. The admin-flag is checked
/// in the database, not only in the token, so an admin, who was degraded, gets no new token.
///
/// # Arguments
///
/// * `user` - The user from the database, for which the token is created
/// * `project_id` - ID of the project, for which the token is created
///
/// # Returns
///
/// * `Ok(ProjectRole)` - The role of the admin within the project, which is `admin`, if the admin
///   is not assigned to the project.
/// * `Err(ErrorResponse::Unauthorized)` - The user is no admin.
/// * `Err(ErrorResponse::NotFound)` - The project doesn't exist.
/// * `Err(ErrorResponse::InternalError)` - The database could not be read.
pub(crate) fn get_admin_role_for_token(
    user: &UserEntry,
    project_id: &String,
) -> Result<ProjectRole, ErrorResponse> {
    if user.is_admin != true.to_string() {
        return Err(ErrorResponse::Unauthorized(
            "Only Admins are allowed to use this endpoint".to_string(),
        ));
    }
    let map_error = |e| match e {
        DbError::NotFound | DbError::PermissionDenied => {
            ErrorResponse::NotFound(format!("Project '{project_id}' not found."))
        }
        DbError::InternalError => {
            ainari_api::report_internal_error!(
                None,
                "Failed to get project '{project_id}' or the role of user '{}' in it from DB",
                user.id
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        }
    };

    project_table::get_auth_project(project_id).map_err(map_error)?;

    // an admin, who is assigned to the project, keeps the role within it
    match user_project_mapping_table::get_mapping(project_id, &user.id) {
        Ok(mapping) => Ok(mapping.role),
        Err(DbError::NotFound) => Ok(ProjectRole::Admin),
        Err(e) => Err(map_error(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_api_structs::user_context::UserContext;
    use chrono::Utc;
    use serial_test::serial;

    fn admin_context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "admin".to_string(),
            project_id: "admin-project".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        }
    }

    fn user(id: &str, is_admin: bool) -> UserEntry {
        UserEntry {
            id: id.to_string(),
            name: id.to_string(),
            is_admin: is_admin.to_string(),
            pw_hash: String::new(),
            salt: String::new(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        }
    }

    #[test]
    #[serial]
    fn test_project_roles_for_tokens() {
        let project_id = format!("test-token-project-{}", uuid::Uuid::new_v4());
        let (member, other) = (
            "test-token-member".to_string(),
            "test-token-other".to_string(),
        );
        project_table::add_new_project(&project_id, "test-project", &admin_context()).unwrap();
        user_project_mapping_table::add_new_mapping(
            &project_id,
            &member,
            ProjectRole::Member,
            &admin_context(),
        )
        .unwrap();

        // the normal tokens are only created for assigned users, also for admins
        assert!(matches!(
            get_project_role_for_token(&member, &project_id),
            Ok(ProjectRole::Member)
        ));
        assert!(matches!(
            get_project_role_for_token(&other, &project_id),
            Err(ErrorResponse::Unauthorized(_))
        ));

        // the admin-tokens only for admins, but also without assignment
        assert!(matches!(
            get_admin_role_for_token(&user(&other, false), &project_id),
            Err(ErrorResponse::Unauthorized(_))
        ));
        assert!(matches!(
            get_admin_role_for_token(&user(&other, true), &project_id),
            Ok(ProjectRole::Admin)
        ));
        assert!(matches!(
            get_admin_role_for_token(&user(&member, true), &project_id),
            Ok(ProjectRole::Member)
        ));
        assert!(matches!(
            get_admin_role_for_token(&user(&other, true), &"test-token-missing".to_string()),
            Err(ErrorResponse::NotFound(_))
        ));

        let _ = user_project_mapping_table::delete_mappings_of_project(&project_id);
        let _ = project_table::delete_project(&project_id, &admin_context());
        let _ = crate::database::quota_table::delete_quota(&project_id, &admin_context());
    }
}
