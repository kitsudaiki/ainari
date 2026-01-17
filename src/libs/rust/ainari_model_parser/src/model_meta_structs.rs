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
use std::collections::HashMap;
use uuid::Uuid;

use ainari_common::constants::*;
use ainari_common::enums::*;
use ainari_common::objects::*;

/// Configuration settings for the neural network model
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Amount of potential reduction in each cycles after a neuron fired
    pub neuron_cooldown: f32,
    /// Number of cycles a neuron remains unresponsive after firing
    pub refractory_time: u32,
    /// Maximum number of hexagons for forming connections between neurons
    pub max_connection_distance: u32,
}

/// Metadata for connections between neurons (axons)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxonMeta {
    /// Starting position of the axon
    pub from: Position,
    /// Ending position of the axon (target neuron)
    pub to: Position,
}

/// Metadata for hexagonal neurons in the neural network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexagonMeta {
    /// Unique identifier for the hexagon
    pub uuid: Uuid,
    /// Position of the hexagon in the network
    pub positon: Position,
    /// Name of the hexagon (can be used for identification)
    pub name: String,

    /// Flag indicating if this hexagon is an input hexagon
    pub is_input: bool,
    /// Flag indicating if this hexagon is an output hexagon
    pub is_output: bool,

    /// UUID of the target hexagon this axon connects to
    pub axon_target: Uuid,

    /// List of possible target hexagons for new connections
    pub possible_hexagon_target_ids: Vec<Uuid>,
    /// Array of neighboring hexagon UUIDs (12 neighbors in a hexagonal grid)
    pub neighbors: [Uuid; 12],
}

impl HexagonMeta {
    /// Creates a new HexagonMeta instance with default values
    ///
    /// # Arguments
    /// * `positon` - The initial position of the hexagon
    ///
    /// # Returns
    /// A new HexagonMeta instance with a generated UUID and default values
    pub fn new(positon: Position) -> Self {
        let new_uuid = Uuid::new_v4();
        HexagonMeta {
            uuid: new_uuid,
            positon,
            name: "".to_string(),

            axon_target: new_uuid, // Default to its own UUID

            is_input: false,
            is_output: false,

            // Initialize neighbors with nil UUIDs
            neighbors: [Uuid::nil(); 12],
            // Initialize possible targets with nil UUIDs and size equal to NUMBER_OF_POSSIBLE_NEXT
            possible_hexagon_target_ids: vec![Uuid::nil(); NUMBER_OF_POSSIBLE_NEXT],
        }
    }
}

/// Metadata for input connections to the neural network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputMeta {
    /// Unique identifier for the input
    pub uuid: Uuid,
    /// UUID of the hexagon this input connects to
    pub hexagon_uuid: Uuid,
    /// Name of the input (can be used for identification)
    pub name: String,
    /// Position of the input in the network
    pub position: Position,
}

impl InputMeta {
    /// Creates a new InputMeta instance
    ///
    /// # Arguments
    /// * `name` - The name of the input
    /// * `position` - The position of the input in the network
    ///
    /// # Returns
    /// A new InputMeta instance with a generated UUID and nil hexagon UUID
    pub fn new(name: String, position: Position) -> Self {
        InputMeta {
            uuid: Uuid::new_v4(),
            hexagon_uuid: Uuid::nil(), // Not connected to a hexagon by default
            name,
            position,
        }
    }
}

/// Metadata for output connections from the neural network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMeta {
    /// Unique identifier for the output
    pub uuid: Uuid,
    /// UUID of the hexagon this output connects to
    pub hexagon_uuid: Uuid,
    /// Name of the output (can be used for identification)
    pub name: String,
    /// Position of the output in the network
    pub position: Position,
    /// Type of output data this connection produces
    pub output_type: OutputType,
}

impl OutputMeta {
    /// Creates a new OutputMeta instance
    ///
    /// # Arguments
    /// * `name` - The name of the output
    /// * `position` - The position of the output in the network
    /// * `output_type` - The type of data this output produces
    ///
    /// # Returns
    /// A new OutputMeta instance with a generated UUID and nil hexagon UUID
    pub fn new(name: String, position: Position, output_type: OutputType) -> Self {
        OutputMeta {
            uuid: Uuid::new_v4(),
            hexagon_uuid: Uuid::nil(), // Not connected to a hexagon by default
            name,
            position,
            output_type,
        }
    }
}

/// Metadata for the entire neural network model
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelMeta {
    /// Unique identifier for the model
    pub uuid: Uuid,
    /// Name of the model
    pub name: String,
    /// Version number of the model
    pub version: i32,
    /// Configuration settings for the model
    pub settings: Settings,
    /// Collection of all hexagons in the model
    pub hexagons: HashMap<Uuid, HexagonMeta>,
    /// List of all connections between neurons (axons)
    pub axons: Vec<AxonMeta>,
    /// List of all input connections to the model
    pub inputs: Vec<InputMeta>,
    /// List of all output connections from the model
    pub outputs: Vec<OutputMeta>,
}
