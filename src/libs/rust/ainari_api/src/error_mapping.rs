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

use uuid::Uuid;

use crate::errors::ErrorResponse;
use crate::omikuji_logging::report_internal_error;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::error::AinariError;

/// Maps AinariError to the appropriate ErrorResponse type.
///
/// This function translates internal AinariError types into the appropriate API error responses.
///
/// # Arguments
///
/// * `e` - An AinariError object to be mapped to an ErrorResponse.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the input AinariError.
pub fn map_ainari_error_to_api_response(
    e: AinariError,
    context: Option<&UserContext>,
) -> ErrorResponse {
    match e {
        AinariError::Unauthorized(msg) => ErrorResponse::Unauthorized(msg),
        AinariError::InvalidInput(msg) => ErrorResponse::BadRequest(msg),
        AinariError::NotFound(msg) => ErrorResponse::NotFound(msg),
        AinariError::Conflict(msg) => ErrorResponse::Conflict(msg),
        AinariError::Forbidden(msg) => ErrorResponse::Forbidden(msg),
        AinariError::InternalError(details) => {
            // the error keeps the user, which it was assigned to before
            let details = match context {
                Some(context) if details.user_id.is_empty() => {
                    details.with_ids(&context.user_id, &context.project_id)
                }
                _ => details,
            };
            report_internal_error(details);
            ErrorResponse::InternalError("Internal Error".to_string())
        }
    }
}

/// Maps a database-error of a register-operation to the appropriate ErrorResponse.
///
/// The concrete error is only logged, because a failed registration is always a problem of the
/// service itself and never something, the caller could fix.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being registered.
/// * `_err` - A DbError enum indicating the type of database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse with the generic internal error message.
pub fn map_db_register_error(
    obj_type: &str,
    _err: enums::DbError,
    context: Option<&UserContext>,
) -> ErrorResponse {
    crate::report_internal_error!(context, "Error while register {obj_type} with in db DB");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Maps database errors for get and delete operations using ID to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being accessed.
/// * `id` - A string slice containing the ID of the object.
/// * `err` - A DbError enum indicating the type of database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_id_get_delete_error(
    obj_type: &str,
    id: &str,
    err: enums::DbError,
    context: Option<&UserContext>,
) -> ErrorResponse {
    match err {
        enums::DbError::InternalError => {
            crate::report_internal_error!(
                context,
                "Error while get {obj_type} with ID '{id}' from DB"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        }
        enums::DbError::NotFound => {
            ErrorResponse::NotFound(format!("{obj_type} with ID '{id}' not found."))
        }
        enums::DbError::PermissionDenied => permission_denied_response(),
    }
}

/// To handle error when requesting a value from the database right after it was
/// added to the database.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being accessed.
/// * `id` - A string slice containing the ID of the object.
/// * `err` - A DbError enum indicating the type of database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// Always InternalError
pub fn map_db_id_get_after_add_error(
    obj_type: &str,
    id: &str,
    _err: enums::DbError,
    context: Option<&UserContext>,
) -> ErrorResponse {
    crate::report_internal_error!(context, "Error while get {obj_type} with ID '{id}' from DB");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// To handle error when requesting a value from the database right after it was
/// added to the database.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being accessed.
/// * `uuid` - A reference to a Uuid object.
/// * `err` - A DbError enum indicating the type of database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// Always InternalError
pub fn map_db_uuid_get_after_add_error(
    obj_type: &str,
    uuid: &Uuid,
    _err: enums::DbError,
    context: Option<&UserContext>,
) -> ErrorResponse {
    crate::report_internal_error!(
        context,
        "Error while get {obj_type} with UUID '{uuid}' from DB"
    );
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Maps database errors for get and delete operations using UUID to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being accessed.
/// * `uuid` - A reference to a Uuid object.
/// * `err` - A DbError enum indicating the type of database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_uuid_get_delete_error(
    obj_type: &str,
    uuid: &Uuid,
    err: enums::DbError,
    context: Option<&UserContext>,
) -> ErrorResponse {
    match err {
        enums::DbError::InternalError => {
            crate::report_internal_error!(
                context,
                "Error while searching {obj_type} with UUID '{uuid}' in DB"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        }
        enums::DbError::NotFound => {
            ErrorResponse::NotFound(format!("{obj_type} with UUID '{uuid}' not found."))
        }
        enums::DbError::PermissionDenied => permission_denied_response(),
    }
}

/// Maps database errors for get and delete operations using UUID to an AinariError.
///
/// Same as `map_db_uuid_get_delete_error`, but for the places, which work with an `AinariError`
/// instead of an `ErrorResponse`. A missing object is reported as invalid input here, because it
/// was addressed by a uuid, which the caller has provided.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being accessed.
/// * `uuid` - A reference to a Uuid object.
/// * `err` - A DbError enum indicating the type of database error.
///
/// # Returns
///
/// An AinariError object corresponding to the database error.
pub fn map_db_uuid_get_delete_ainari_error(
    obj_type: &str,
    uuid: &Uuid,
    err: enums::DbError,
) -> AinariError {
    match err {
        enums::DbError::InternalError => AinariError::internal(format!(
            "Error while searching {obj_type} with UUID '{uuid}' in DB"
        )),
        enums::DbError::NotFound => {
            AinariError::InvalidInput(format!("{obj_type} with UUID '{uuid}' not found."))
        }
        enums::DbError::PermissionDenied => {
            AinariError::Forbidden("Permission denied.".to_string())
        }
    }
}

/// Maps database errors for list operations to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being listed.
/// * `e` - A diesel::result::Error object indicating the database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_list_error(
    obj_type: &str,
    e: diesel::result::Error,
    context: Option<&UserContext>,
) -> ErrorResponse {
    crate::report_internal_error!(context, "Failed to list {obj_type} with error: '{e}'");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Maps database errors for count operations to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `obj_type` - A string slice describing the type of object being counted.
/// * `e` - A diesel::result::Error object indicating the database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse object corresponding to the database error.
pub fn map_db_count_error(
    obj_type: &str,
    e: diesel::result::Error,
    context: Option<&UserContext>,
) -> ErrorResponse {
    crate::report_internal_error!(context, "Failed to count {obj_type} with error: '{e}'");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Response for a request, which was blocked by the database, because the user of the context is
/// not allowed to change the entry, for example because it is only observer of the project.
///
/// # Returns
///
/// An ErrorResponse with status 403.
pub fn permission_denied_response() -> ErrorResponse {
    ErrorResponse::Forbidden("Permission denied.".to_string())
}

/// Maps database errors for adding or changing operations to appropriate ErrorResponse.
///
/// A blocked write-access is reported to the client, all other errors are only logged, because
/// they must not be exposed to the client.
///
/// # Arguments
///
/// * `action` - A string slice describing the action, which has failed.
/// * `e` - A diesel::result::Error object indicating the database error.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse with status 403 for a blocked write-access, else the generic internal error.
pub fn map_db_write_error(
    action: &str,
    e: diesel::result::Error,
    context: Option<&UserContext>,
) -> ErrorResponse {
    if enums::is_permission_denied(&e) {
        return permission_denied_response();
    }
    crate::report_internal_error!(context, "Failed to {action} with error: '{e}'");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Maps internal errors, which must not be exposed to the client, to appropriate ErrorResponse.
///
/// # Arguments
///
/// * `action` - A string slice describing the action, which has failed.
/// * `e` - The error, which occurred while performing the action.
/// * `context` - Context of the request, whose user the error is assigned to, or `None` outside
///   of a request.
///
/// # Returns
///
/// An ErrorResponse object with the generic internal error message.
pub fn map_internal_error(
    action: &str,
    e: impl std::fmt::Display,
    context: Option<&UserContext>,
) -> ErrorResponse {
    crate::report_internal_error!(context, "Failed to {action} with error: '{e}'");
    ErrorResponse::InternalError("Internal Error".to_string())
}
