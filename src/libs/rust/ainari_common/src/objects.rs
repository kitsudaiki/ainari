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

use super::constants::UNINIT_POINT_32;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl Position {
    pub fn new() -> Self {
        Position {
            x: UNINIT_POINT_32,
            y: UNINIT_POINT_32,
            z: UNINIT_POINT_32,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.x != UNINIT_POINT_32 && self.y != UNINIT_POINT_32 && self.z != UNINIT_POINT_32
    }

    pub fn to_string(&self) -> String {
        format!("[ {} , {} , {} ]", self.x, self.y, self.z)
    }
}
