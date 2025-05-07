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
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub enum OutputType {
    PlainOutput = 0,
    BoolOutput = 1,
    IntOutput = 2,
    FloatOutput = 3,
}
impl Default for OutputType {
    fn default() -> Self { OutputType::PlainOutput }
}

#[derive(Debug)]
pub struct Settings {
    pub neuron_cooldown: f32,
    pub refractory_time: u32,
    pub max_connection_distance: u32,
}

#[derive(Debug, PartialEq)]
pub struct Position(pub u32, pub u32, pub u32);

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.0, self.1, self.2)
    }
}

#[derive(Debug)]
pub struct AxonMeta {
    pub from: Position,
    pub to: Position,
}

#[derive(Debug)]
pub struct InputMeta {
    pub name: String,
    pub pos: Position,
}

#[derive(Debug)]
pub struct OutputMeta {
    pub name: String,
    pub pos: Position,
    pub output_type: OutputType,
}

#[derive(Debug)]
pub struct ClusterMeta {
    pub version: i32,
    pub settings: Settings,
    pub hexagons: Vec<Position>,
    pub axons: Vec<AxonMeta>,
    pub inputs: Vec<InputMeta>,
    pub outputs: Vec<OutputMeta>,
}
