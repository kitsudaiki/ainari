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

use std::collections::HashSet;
use tokio::fs;
use uuid::Uuid;

use crate::error_mapping::permission_denied_response;
use crate::errors::ErrorResponse;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

/// Creates a directory and all necessary parent directories asynchronously.
///
/// # Arguments
///
/// * `path` - A string slice that holds the path to the directory to be created.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// * `Ok(())` if the directory was created successfully.
/// * `Err(ErrorResponse::InternalError)` if an error occurred during directory creation.
pub async fn create_directory_api(
    path: &String,
    context: Option<&UserContext>,
) -> Result<(), ErrorResponse> {
    match fs::create_dir_all(&path).await {
        Ok(_) => (),
        Err(e) => {
            crate::report_internal_error!(context, "Failed to directory '{path}' with error: {e}");
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
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// * `Ok(Uuid)` if the conversion was successful.
/// * `Err(ErrorResponse::InternalError)` if an error occurred during the conversion.
pub fn convert_uuid(uuid: &String, context: Option<&UserContext>) -> Result<Uuid, ErrorResponse> {
    let uuid = match Uuid::parse_str(uuid) {
        Ok(uuid) => uuid,
        Err(e) => {
            crate::report_internal_error!(
                context,
                "Failed to convert string '{uuid}' into uuid with error: '{e}'"
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(uuid)
}

/// Checks if an object with the given ID already exists in the database.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being checked.
/// * `id` - A string slice containing the ID of the object.
/// * `ret` - A Result object containing either the object or a DbError.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
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
    context: Option<&UserContext>,
) -> Result<(), ErrorResponse> {
    match ret {
        Ok(_) => {
            let msg = format!("{obj_type} with ID '{id}' already exist.");
            return Err(ErrorResponse::Conflict(msg));
        }
        Err(enums::DbError::InternalError) => {
            crate::report_internal_error!(
                context,
                "Error while checking, if {obj_type} with ID '{id}' already exists in DB"
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
        Err(enums::DbError::NotFound) => {
            // it is desired, that the object not already exist, so this error will be ignored
        }
        Err(enums::DbError::PermissionDenied) => {
            return Err(permission_denied_response());
        }
    };

    Ok(())
}

/// Checks, that the user is allowed to access the resources of a project, which is given by the
/// request and not by the token. Admins can access every project, all other users only the project
/// of their token.
///
/// # Arguments
///
/// * `project_id` - The ID of the project
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(())` - The user is allowed
/// * `Err(ErrorResponse::Forbidden)` - The project is not the one of the user
pub fn check_project_access(project_id: &str, context: &UserContext) -> Result<(), ErrorResponse> {
    if context.is_admin != true.to_string() && context.project_id != project_id {
        return Err(permission_denied_response());
    }
    Ok(())
}

/// Checks, that no UUID is used twice within an import of resources, for example because of a
/// wrong mapping of the UUIDs.
///
/// # Arguments
///
/// * `uuids` - The UUIDs of all imported resources
///
/// # Returns
///
/// * `Ok(())` - All UUIDs are unique
/// * `Err(ErrorResponse::BadRequest)` - A UUID is used more than once
pub fn check_unique_uuids<'a>(
    uuids: impl IntoIterator<Item = &'a Uuid>,
) -> Result<(), ErrorResponse> {
    let mut seen = HashSet::new();
    for uuid in uuids {
        if !seen.insert(uuid) {
            return Err(ErrorResponse::BadRequest(format!(
                "UUID '{uuid}' is used more than once within the import."
            )));
        }
    }
    Ok(())
}

/// Checks, if a resource can be imported into a project under a UUID.
///
/// A UUID, which is used by another project, is never taken over, so an import can't change the
/// resources of another project. An existing resource of the project is only overwritten with
/// `force`.
///
/// # Arguments
///
/// * `existing` - Project of the entry, which already uses the UUID, if there is one
/// * `project_id` - The project, into which the resource is imported
/// * `force` - Overwrite an existing resource of the project
/// * `kind` - Kind of the resource for the error-message
/// * `uuid` - The UUID of the resource
///
/// # Returns
///
/// * `Ok(bool)` - The resource can be imported, true if an existing resource is overwritten
/// * `Err(ErrorResponse::Conflict)` - The UUID is used by another project, or by an existing
///   resource of the project without `force`
pub fn check_import_target(
    existing: Option<&str>,
    project_id: &str,
    force: bool,
    kind: &str,
    uuid: &Uuid,
) -> Result<bool, ErrorResponse> {
    match existing {
        None => Ok(false),
        Some(existing_project) if existing_project != project_id => {
            Err(ErrorResponse::Conflict(format!(
                "{kind} with UUID '{uuid}' can't be imported, because the UUID is already used by \
                 another project."
            )))
        }
        Some(_) if !force => Err(ErrorResponse::Conflict(format!(
            "{kind} with UUID '{uuid}' already exists. Use 'force' to overwrite it."
        ))),
        Some(_) => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_unique_uuids() {
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        assert!(check_unique_uuids(&[a, b]).is_ok());
        assert!(check_unique_uuids(&[]).is_ok());
        assert!(matches!(
            check_unique_uuids(&[a, b, a]),
            Err(ErrorResponse::BadRequest(_))
        ));
    }

    #[test]
    fn test_check_import_target() {
        let uuid = Uuid::new_v4();
        let check =
            |existing, force| check_import_target(existing, "project", force, "Secret", &uuid);

        // free UUID
        assert!(matches!(check(None, false), Ok(false)));
        // existing entry of the project only with force
        assert!(matches!(
            check(Some("project"), false),
            Err(ErrorResponse::Conflict(_))
        ));
        assert!(matches!(check(Some("project"), true), Ok(true)));
        // never the entry of another project
        for force in [false, true] {
            assert!(matches!(
                check(Some("other"), force),
                Err(ErrorResponse::Conflict(_))
            ));
        }
    }
}
