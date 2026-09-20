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

// ==================================================================================================

/// Result of a database-lookup, which reduces the diesel-errors to the two cases, the callers
/// actually have to distinguish.
pub enum DbError {
    NotFound,
    InternalError,
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
