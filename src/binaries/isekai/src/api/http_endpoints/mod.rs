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

pub mod backup;
pub mod import;
pub mod job;
pub mod restore;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::ProjectRole;

/// Checks if the user is an admin of the whole deployment.
fn is_global_admin(context: &UserContext) -> bool {
    context.is_admin == true.to_string()
}

/// Checks if the user is allowed to administrate the backups of its project, like deleting them
/// or restoring them.
///
/// # Arguments
///
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(())` - The user is admin of the project or of the whole deployment
/// * `Err(ErrorResponse::Forbidden)` - The user is only member or observer of the project
fn check_project_admin(context: &UserContext) -> Result<(), ErrorResponse> {
    if is_global_admin(context) || context.project_role == ProjectRole::Admin.as_str() {
        return Ok(());
    }
    Err(ErrorResponse::Forbidden(
        "Only admins of the project are allowed to do this.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(is_admin: bool, role: ProjectRole) -> UserContext {
        UserContext {
            token: String::new(),
            user_id: "user".to_string(),
            project_id: "project".to_string(),
            is_admin: is_admin.to_string(),
            project_role: role.to_string(),
        }
    }

    #[test]
    fn test_check_project_admin() {
        assert!(check_project_admin(&context(false, ProjectRole::Admin)).is_ok());
        assert!(check_project_admin(&context(true, ProjectRole::Observer)).is_ok());
        for role in [ProjectRole::Member, ProjectRole::Observer] {
            assert!(matches!(
                check_project_admin(&context(false, role)),
                Err(ErrorResponse::Forbidden(_))
            ));
        }
    }
}
