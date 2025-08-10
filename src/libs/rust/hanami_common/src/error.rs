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

use std::fmt;

#[derive(Debug)]
pub enum HanamiError {
    InputError(String),
    Error(String),
}

impl fmt::Display for HanamiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            HanamiError::InputError(ref msg) => write!(f, "Input-error: {msg}"),
            HanamiError::Error(ref msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl PartialEq<&str> for HanamiError {
    fn eq(&self, other: &&str) -> bool {
        match self {
            HanamiError::InputError(s) | HanamiError::Error(s) => s == other,
        }
    }
}

impl std::error::Error for HanamiError {}

