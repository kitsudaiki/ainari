// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimim.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[derive(Debug, PartialEq)]
pub enum OutputType {
    UNKNOWN_TYPE = 0,
    PLAIN_OUTPUT = 1,
    BOOL_OUTPUT = 2,
    INT_OUTPUT = 3,
    FLOAT_OUTPUT = 4,
}

#[derive(Debug)]
pub struct Settings {
    pub neuron_cooldown: f32,
    pub refractory_time: u32,
    pub max_connection_distance: u32,
}

#[derive(Debug, PartialEq)]
pub struct Position(pub i32, pub i32, pub i32);

#[derive(Debug)]
pub struct Axon {
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
    pub axons: Vec<Axon>,
    pub inputs: Vec<InputMeta>,
    pub outputs: Vec<OutputMeta>,
}
