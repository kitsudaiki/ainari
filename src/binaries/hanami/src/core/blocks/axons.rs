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

use std::sync::{Arc, Mutex};
use uuid::Uuid;

use hanami_common::constants::*;

use super::block_trait::*;

// ==================================================================================================

#[derive(Default, Copy, Clone)]
pub struct Axon {
    pub potential: f32,
    pub delta: f32,
}

// ==================================================================================================

#[derive(Clone)]
pub struct AxonSection {
    pub axons: [Axon; BLOCK_DIM],

    pub cluster_uuid: Uuid,

    pub source_hexagon_uuid: Uuid,
    pub target_hexagon_uuid: Uuid,

    pub source_block_uuid: Uuid,
    pub target_block_uuid: Uuid,

    pub target_pos: u8,
    pub source_pos: u8,

    pub source_block: Option<Arc<Mutex<dyn Block>>>,
    pub target_block: Option<Arc<Mutex<dyn Block>>>,
}

impl AxonSection {
    pub fn default() -> Self {
        AxonSection {
            axons: std::array::from_fn(|_| Axon::default()),
            cluster_uuid: Uuid::nil(),
            source_hexagon_uuid: Uuid::nil(),
            target_hexagon_uuid: Uuid::nil(),
            source_block_uuid: Uuid::nil(),
            target_block_uuid: Uuid::nil(),
            target_pos: UNINIT_STATE_8,
            source_pos: UNINIT_STATE_8,
            source_block: None,
            target_block: None,
        }
    }
}

// ==================================================================================================
