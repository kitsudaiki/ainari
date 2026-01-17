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
use serde_big_array::BigArray;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ainari_common::constants::*;

use super::block_trait::*;

// ==================================================================================================

/// Represents a single axon in the neural network.
/// Contains potential and delta values used in neural computations.
#[derive(Default, Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Axon {
    /// The potential value of the axon, used in neural computations.
    pub potential: f32,
    /// The delta value of the axon, used in neural computations.
    pub delta: f32,
}

// ==================================================================================================

/// Represents a collection of axons within a block.
/// Contains an array of axons with a fixed size defined by BLOCK_DIM.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxonData {
    /// Array of axons with size defined by BLOCK_DIM constant.
    /// Uses BigArray for efficient serialization of large arrays.
    #[serde(with = "BigArray")]
    pub axons: [Axon; BLOCK_DIM],
}

impl AxonData {
    /// Creates a new AxonData instance with default values for all axons.
    ///
    /// # Returns
    /// An AxonData instance with all axons set to their default values.
    pub fn default() -> Self {
        AxonData {
            axons: std::array::from_fn(|_| Axon::default()),
        }
    }
}

// ==================================================================================================

/// Represents a section of axons connecting two blocks in the neural network.
/// Contains information about the connection and the axon data.
#[derive(Debug, Serialize, Deserialize)]
pub struct AxonSection {
    /// The axon data contained in this section.
    pub data: AxonData,

    /// UUID of the model this axon section belongs to.
    pub model_uuid: Uuid,

    /// UUID of the source hexagon this axon section originates from.
    pub source_hexagon_uuid: Uuid,
    /// UUID of the target hexagon this axon section connects to.
    pub target_hexagon_uuid: Uuid,

    /// UUID of the source block this axon section originates from.
    pub source_block_uuid: Uuid,
    /// UUID of the target block this axon section connects to.
    pub target_block_uuid: Uuid,

    /// Position within the target block.
    pub target_pos: u8,
    /// Position within the source block.
    pub source_pos: u8,
    /// Flag indicating whether the axon section is completed.
    pub done: bool,

    /// Reference to the source block (not serialized).
    #[serde(skip)]
    pub source_block: Option<Arc<Mutex<dyn Block>>>,
    /// Reference to the target block (not serialized).
    #[serde(skip)]
    pub target_block: Option<Arc<Mutex<dyn Block>>>,
}

impl Clone for AxonSection {
    /// Creates a deep copy of the AxonSection.
    ///
    /// # Returns
    /// A new AxonSection with the same values as the original.
    fn clone(&self) -> Self {
        Self {
            data: self.data,

            model_uuid: self.model_uuid,

            source_hexagon_uuid: self.source_hexagon_uuid,
            target_hexagon_uuid: self.target_hexagon_uuid,

            source_block_uuid: self.source_block_uuid,
            target_block_uuid: self.target_block_uuid,

            target_pos: self.target_pos,
            source_pos: self.source_pos,
            done: self.done,

            source_block: self.source_block.clone(),
            target_block: self.target_block.clone(),
        }
    }
}

impl AxonSection {
    /// Creates a new AxonSection with default values.
    ///
    /// # Returns
    /// An AxonSection with all fields set to their default values.
    pub fn default() -> Self {
        AxonSection {
            data: AxonData::default(),
            model_uuid: Uuid::nil(),
            source_hexagon_uuid: Uuid::nil(),
            target_hexagon_uuid: Uuid::nil(),
            source_block_uuid: Uuid::nil(),
            target_block_uuid: Uuid::nil(),
            target_pos: UNINIT_STATE_8,
            source_pos: UNINIT_STATE_8,
            source_block: None,
            target_block: None,
            done: false,
        }
    }
}

impl PartialEq for AxonSection {
    /// Compares two AxonSection instances for equality.
    ///
    /// # Arguments
    /// * `other` - The other AxonSection to compare with.
    ///
    /// # Returns
    /// true if all fields are equal, false otherwise.
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
            && self.model_uuid == other.model_uuid
            && self.source_hexagon_uuid == other.source_hexagon_uuid
            && self.target_hexagon_uuid == other.target_hexagon_uuid
            && self.source_block_uuid == other.source_block_uuid
            && self.target_block_uuid == other.target_block_uuid
            && self.target_pos == other.target_pos
            && self.source_pos == other.source_pos
            && self.done == other.done
    }
}

// ==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests serialization and deserialization of AxonSection.
    #[test]
    fn test_serialize_deserialize() {
        // Create a test AxonSection with some modified values
        let mut original = AxonSection {
            data: AxonData::default(),
            model_uuid: Uuid::new_v4(),
            source_hexagon_uuid: Uuid::new_v4(),
            target_hexagon_uuid: Uuid::new_v4(),
            source_block_uuid: Uuid::new_v4(),
            target_block_uuid: Uuid::new_v4(),
            target_pos: 42,
            source_pos: 43,
            source_block: None,
            target_block: None,
            done: false,
        };
        // Modify specific axon values for testing
        original.data.axons[42].potential = 123.0f32;
        original.data.axons[42].delta = 124.0f32;

        // Serialize the AxonSection
        let cfg = bincode::config::standard();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: AxonSection = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;

        // Print the size of the serialized data
        println!("size: {}", serialized.len());

        // Verify that the deserialized data matches the original
        assert_eq!(original, deserialized);
    }
}
