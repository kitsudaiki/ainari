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

use serde::{Deserialize, Serialize};
use std::fmt;

// ==================================================================================================

pub enum DbError {
    NotFound,
    InternalError,
}

// ==================================================================================================

#[repr(i32)]
#[derive(Debug, PartialEq)]
pub enum ReturnStatus {
    OK = 0,
    InvalidInput = 1,
    Error = 2,
}

impl ReturnStatus {
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

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, Default)]
pub enum OutputType {
    #[default]
    PlainOutput = 0,
    BoolOutput = 1,
    IntOutput = 2,
    FloatOutput = 3,
}

// ==================================================================================================

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjectType {
    Unknown,
    ClusterMeta,
    HexagonData,
    InputBlock,
    CoreBlock,
    OutputBlock,
    OutputBuffer,
}

impl ObjectType {
    pub fn to_u8(&self) -> u8 {
        match self {
            ObjectType::Unknown => 0,
            ObjectType::ClusterMeta => 1,
            ObjectType::HexagonData => 2,
            ObjectType::InputBlock => 3,
            ObjectType::CoreBlock => 4,
            ObjectType::OutputBlock => 5,
            ObjectType::OutputBuffer => 6,
        }
    }

    pub fn from_u8(value: u8) -> Option<ObjectType> {
        match value {
            0 => Some(ObjectType::Unknown),
            1 => Some(ObjectType::ClusterMeta),
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
