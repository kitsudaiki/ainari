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

use std::time::Duration;

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;

use crate::core::project_handling;
use crate::database::project_table;
use crate::database::user_project_mapping_table;
use crate::database::user_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::ProjectRole;

/// Time to wait after the users of the default-project were made observers, so requests, which
/// were already running with the old role, are finished before the resources are checked again.
const OBSERVER_GRACE_PERIOD: Duration = Duration::from_secs(1);

#[api_operation(
    tag = "user",
    summary = "Delete user",
    description = r###"Delete a user together with its default-project. The user can only be deleted, if its default-project contains no resources anymore. To prevent the creation of new resources during the delete, all users of the default-project are made observers before. The user is removed from all other projects, but these projects are never deleted. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn delete_user_admin(
    user_id: Path<String>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    check_admin_context(&context)?;

    if context.user_id == user_id.to_string() {
        return Err(ErrorResponse::Conflict(
            "A user can not delete himself.".to_string(),
        ));
    }

    // check if user exist
    user_table::get_user(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", &user_id, e))?;

    // only the default-project is deleted together with the user, so only its resources are
    // checked. Other projects are not touched.
    let default_project_id = project_table::default_project_id(&user_id);
    check_default_project_empty(&user_id, &default_project_id, &context).await?;

    // All users of the default-project, which can create resources, are made observers, so no
    // new resource can be created in the project anymore. After a short wait, for requests,
    // which were still running with the old role, the project is checked again.
    let demoted_users = demote_to_observers(&default_project_id, &context)?;
    actix_rt::time::sleep(OBSERVER_GRACE_PERIOD).await;
    if let Err(e) = check_default_project_empty(&user_id, &default_project_id, &context).await {
        restore_roles(&default_project_id, &demoted_users, &context);
        return Err(e);
    }

    // delete user from database
    user_table::delete_user(&user_id, &context)
        .map_err(|e| map_db_id_get_delete_error("user", &user_id, e))?;

    // the user is removed from all projects, but only the default-project is deleted
    user_project_mapping_table::delete_mappings_of_user(&user_id)
        .map_err(|e| map_db_id_get_delete_error("user-project-mapping", &user_id, e))?;

    project_handling::delete_project_completely(&default_project_id, &context)?;

    Ok(NoContent)
}

/// Checks, that the default-project of a user doesn't contain any resources.
///
/// # Arguments
///
/// * `user_id` - The ID of the user, which should be deleted
/// * `default_project_id` - The ID of the default-project of the user
/// * `context` - The user context, whose token is used for the requests
///
/// # Returns
///
/// Ok(()) if the project is empty, or a conflict, which lists the remaining resources
async fn check_default_project_empty(
    user_id: &str,
    default_project_id: &str,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let resources =
        project_handling::list_resources_of_project(default_project_id, context).await?;
    if !resources.is_empty() {
        return Err(ErrorResponse::Conflict(format!(
            "User '{user_id}' can not be deleted, because its default-project \
             '{default_project_id}' still contains resources ({}). The resources have to be \
             deleted first.",
            resources.join(", ")
        )));
    }
    Ok(())
}

/// Makes all admins and members of a project observers.
///
/// # Arguments
///
/// * `project_id` - The ID of the project
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// The IDs of the changed users together with their old roles, to restore them in case of an
/// error.
fn demote_to_observers(
    project_id: &String,
    context: &UserContext,
) -> Result<Vec<(String, ProjectRole)>, ErrorResponse> {
    let mappings = user_project_mapping_table::list_mappings_of_project(project_id)
        .map_err(|e| map_db_list_error("user-project-mappings", e))?;

    let mut demoted_users = Vec::new();
    for mapping in mappings {
        if !matches!(mapping.role, ProjectRole::Admin | ProjectRole::Member) {
            continue;
        }
        if let Err(e) = user_project_mapping_table::set_mapping_role(
            project_id,
            &mapping.user_id,
            ProjectRole::Observer,
            context,
        ) {
            restore_roles(project_id, &demoted_users, context);
            return Err(map_db_id_get_delete_error(
                "user-project-mapping",
                &mapping.user_id,
                e,
            ));
        }
        demoted_users.push((mapping.user_id, mapping.role));
    }

    Ok(demoted_users)
}

/// Restores the old roles of users within a project, after the delete was aborted.
///
/// Errors are only logged, because the delete has already failed and its error is returned.
///
/// # Arguments
///
/// * `project_id` - The ID of the project
/// * `users` - The IDs of the users together with their old roles
/// * `context` - The user context containing authentication information
fn restore_roles(project_id: &String, users: &[(String, ProjectRole)], context: &UserContext) {
    for (user_id, role) in users {
        if user_project_mapping_table::set_mapping_role(project_id, user_id, *role, context)
            .is_err()
        {
            log::error!(
                "Failed to restore the role '{role}' of user '{user_id}' in project '{project_id}'"
            );
        }
    }
}
