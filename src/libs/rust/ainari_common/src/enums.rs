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

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

// ==================================================================================================

/// Result of a database-lookup, which reduces the diesel-errors to the two cases, the callers
/// actually have to distinguish.
pub enum DbError {
    NotFound,
    InternalError,
    /// The user of the context is not allowed to change the entry, for example because it is
    /// only observer of the project
    PermissionDenied,
}

/// Message of the error, which marks a blocked write-access within the database-functions.
const PERMISSION_DENIED_MSG: &str = "Permission denied.";

/// Creates the error, which the database-functions with a `QueryResult` return, if the user of
/// the context is not allowed to change the entry.
///
/// # Returns
///
/// A diesel-error, which is recognized by `is_permission_denied`.
pub fn permission_denied_error() -> diesel::result::Error {
    diesel::result::Error::DatabaseError(
        diesel::result::DatabaseErrorKind::CheckViolation,
        Box::new(PERMISSION_DENIED_MSG.to_string()),
    )
}

/// Checks if a diesel-error was created by `permission_denied_error`.
///
/// # Arguments
///
/// * `error` - Error returned by a database-function
///
/// # Returns
///
/// True, if the error marks a blocked write-access, else false.
pub fn is_permission_denied(error: &diesel::result::Error) -> bool {
    match error {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::CheckViolation,
            info,
        ) => info.message() == PERMISSION_DENIED_MSG,
        _ => false,
    }
}

// ==================================================================================================

/// Status-code of a call into the former C++-backend.
///
/// Leftover of the previous project-direction and currently unused, because no C++-code is called
/// anymore.
#[repr(i32)]
#[derive(Debug, PartialEq)]
pub enum ReturnStatus {
    OK = 0,
    InvalidInput = 1,
    Error = 2,
}

impl ReturnStatus {
    /// Converts the raw integer of a C++-call into a `ReturnStatus`.
    ///
    /// # Arguments
    ///
    /// * `val` - Raw status-value as returned by the C++-side
    ///
    /// # Returns
    ///
    /// The matching status, or `Error` for every value, which is not known.
    pub fn from_cpp(val: i32) -> Self {
        match val {
            0 => ReturnStatus::OK,
            1 => ReturnStatus::InvalidInput,
            2 => ReturnStatus::Error,
            _ => ReturnStatus::Error, // fallback for unexpected values
        }
    }
}

impl fmt::Display for ReturnStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

// ==================================================================================================

/// Type of an output-value of the former neural-network-backend.
///
/// Leftover of the previous project-direction and currently unused.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, Default)]
pub enum OutputType {
    #[default]
    PlainOutput = 0,
    BoolOutput = 1,
    IntOutput = 2,
    FloatOutput = 3,
}

// ==================================================================================================

/// Type-tag of an object within a binary file of the former neural-network-backend.
///
/// Leftover of the previous project-direction and currently unused.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjectType {
    Unknown,
    InstanceMeta,
    HexagonData,
    InputBlock,
    CoreBlock,
    OutputBlock,
    OutputBuffer,
}

impl ObjectType {
    /// Converts the type into the byte, which represents it within a binary file.
    ///
    /// # Returns
    ///
    /// The byte-representation of the type.
    pub fn to_u8(&self) -> u8 {
        match self {
            ObjectType::Unknown => 0,
            ObjectType::InstanceMeta => 1,
            ObjectType::HexagonData => 2,
            ObjectType::InputBlock => 3,
            ObjectType::CoreBlock => 4,
            ObjectType::OutputBlock => 5,
            ObjectType::OutputBuffer => 6,
        }
    }

    /// Converts a byte of a binary file back into its type.
    ///
    /// # Arguments
    ///
    /// * `value` - Byte-representation of the type
    ///
    /// # Returns
    ///
    /// The matching type, or None, if the byte belongs to no known type.
    pub fn from_u8(value: u8) -> Option<ObjectType> {
        match value {
            0 => Some(ObjectType::Unknown),
            1 => Some(ObjectType::InstanceMeta),
            2 => Some(ObjectType::HexagonData),
            3 => Some(ObjectType::InputBlock),
            4 => Some(ObjectType::CoreBlock),
            5 => Some(ObjectType::OutputBlock),
            6 => Some(ObjectType::OutputBuffer),
            _ => None,
        }
    }
}

// ==================================================================================================

/// Role of a user within a project.
///
/// The role is stored as lowercase string, which is the same representation as used by serde.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProjectRole {
    Admin,
    Member,
    Observer,
}

impl ProjectRole {
    /// Converts the role into its string-representation.
    ///
    /// # Returns
    ///
    /// The lowercase name of the role.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectRole::Admin => "admin",
            ProjectRole::Member => "member",
            ProjectRole::Observer => "observer",
        }
    }
}

impl fmt::Display for ProjectRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ProjectRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(ProjectRole::Admin),
            "member" => Ok(ProjectRole::Member),
            "observer" => Ok(ProjectRole::Observer),
            _ => Err(format!("Unknown project-role '{s}'")),
        }
    }
}

// ==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_permission_denied() {
        assert!(is_permission_denied(&permission_denied_error()));

        // other errors, also with the same kind, are not taken as blocked access
        assert!(!is_permission_denied(&diesel::result::Error::NotFound));
        let other_check = diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::CheckViolation,
            Box::new("CHECK constraint failed".to_string()),
        );
        assert!(!is_permission_denied(&other_check));
    }
}
