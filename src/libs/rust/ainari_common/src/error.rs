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

use serde::{Deserialize, Serialize};
use std::fmt;

/// Content of an internal error.
///
/// Beside the message, it contains the user and the project, whose request caused the error, so
/// the error can be assigned to them in the error-log of omikuji. They are empty, as long as they
/// are not known, for example in the libraries, and filled in, where the error reaches the
/// context of the request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InternalErrorDetails {
    pub message: String,
    pub user_id: String,
    pub project_id: String,
}

impl InternalErrorDetails {
    /// Creates the details of an error, which is not assigned to a user yet.
    pub fn new(message: impl Into<String>) -> Self {
        InternalErrorDetails {
            message: message.into(),
            user_id: String::new(),
            project_id: String::new(),
        }
    }

    /// Assigns the error to a user and project.
    pub fn with_ids(mut self, user_id: &str, project_id: &str) -> Self {
        self.user_id = user_id.to_string();
        self.project_id = project_id.to_string();
        self
    }
}

impl fmt::Display for InternalErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug)]
pub enum AinariError {
    Unauthorized(String),
    InvalidInput(String),
    /// The requested object doesn't exist in the called service
    NotFound(String),
    /// The request conflicts with existing resources of the called service, like a quota
    Conflict(String),
    /// The user is not allowed to change the object, for example because it is only observer
    Forbidden(String),
    InternalError(InternalErrorDetails),
}

impl AinariError {
    /// Creates an internal error, which is not assigned to a user yet.
    pub fn internal(message: impl Into<String>) -> Self {
        AinariError::InternalError(InternalErrorDetails::new(message))
    }
}

impl fmt::Display for AinariError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AinariError::Unauthorized(ref msg) => write!(f, "Unauthorized: {msg}"),
            AinariError::InvalidInput(ref msg) => write!(f, "Invalid input: {msg}"),
            AinariError::NotFound(ref msg) => write!(f, "Not found: {msg}"),
            AinariError::Conflict(ref msg) => write!(f, "Conflict: {msg}"),
            AinariError::Forbidden(ref msg) => write!(f, "Forbidden: {msg}"),
            AinariError::InternalError(ref details) => write!(f, "Internal error: {details}"),
        }
    }
}

impl From<Box<dyn std::error::Error>> for AinariError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        AinariError::internal(error.to_string())
    }
}

impl PartialEq<&str> for AinariError {
    fn eq(&self, other: &&str) -> bool {
        match self {
            AinariError::Unauthorized(s)
            | AinariError::InvalidInput(s)
            | AinariError::NotFound(s)
            | AinariError::Conflict(s)
            | AinariError::Forbidden(s) => s == other,
            AinariError::InternalError(details) => details.message == *other,
        }
    }
}

impl std::error::Error for AinariError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_error_details() {
        let error = AinariError::internal("database is gone");
        let AinariError::InternalError(details) = &error else {
            panic!("not an internal error");
        };
        // created without a user, which is assigned later
        assert_eq!(details.message, "database is gone");
        assert!(details.user_id.is_empty());
        assert!(details.project_id.is_empty());
        assert_eq!(error.to_string(), "Internal error: database is gone");
        assert!(error == "database is gone");

        let details = details.clone().with_ids("user-1", "project-1");
        assert_eq!(details.user_id, "user-1");
        assert_eq!(details.project_id, "project-1");
    }

    #[test]
    fn test_internal_error_details_as_json() {
        let details = InternalErrorDetails::new("failed").with_ids("user-1", "project-1");
        let json = serde_json::to_value(&details).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"message": "failed", "user_id": "user-1", "project_id": "project-1"})
        );
    }
}
