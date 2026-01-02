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

#[derive(Default, Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Axon {
    pub potential: f32,
    pub delta: f32,
}

// ==================================================================================================

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxonData {
    #[serde(with = "BigArray")]
    pub axons: [Axon; BLOCK_DIM],
}

impl AxonData {
    pub fn default() -> Self {
        AxonData {
            axons: std::array::from_fn(|_| Axon::default()),
        }
    }
}

// ==================================================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct AxonSection {
    pub data: AxonData,

    pub cluster_uuid: Uuid,

    pub source_hexagon_uuid: Uuid,
    pub target_hexagon_uuid: Uuid,

    pub source_block_uuid: Uuid,
    pub target_block_uuid: Uuid,

    pub target_pos: u8,
    pub source_pos: u8,
    pub done: bool,

    #[serde(skip)]
    pub source_block: Option<Arc<Mutex<dyn Block>>>,
    #[serde(skip)]
    pub target_block: Option<Arc<Mutex<dyn Block>>>,
}

impl Clone for AxonSection {
    fn clone(&self) -> Self {
        Self {
            data: self.data,

            cluster_uuid: self.cluster_uuid,

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
    pub fn default() -> Self {
        AxonSection {
            data: AxonData::default(),
            cluster_uuid: Uuid::nil(),
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
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
            && self.cluster_uuid == other.cluster_uuid
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

    #[test]
    fn test_serialize_deserialize() {
        let mut original = AxonSection {
            data: AxonData::default(),
            cluster_uuid: Uuid::new_v4(),
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
        original.data.axons[42].potential = 123.0f32;
        original.data.axons[42].delta = 124.0f32;

        let cfg = bincode::config::standard();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: AxonSection = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;

        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
