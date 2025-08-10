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

use uuid::Uuid;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

use super::axons::*;
use super::block_trait::*;
use super::super::processing::worker_queue::*;
use super::block_io::*;

use crate::core::cluster_handler::*;

use ainari_common::constants::*;
use ainari_common::enums::*;

// ==================================================================================================

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct InputBlock {
    pub uuid: Uuid,
    pub hexagon_uuid: Uuid,
    pub cluster_uuid: Uuid,

    pub block_io: BlockIoBuffer,

    pub name: String,
    pub input_links: Vec<u64>,
    pub fill_position: u64,

    pub local_finish_counter: u64,
    #[serde(skip, default = "init_finish_counter")]
    pub finish_counter: Arc<Mutex<FinishCounter>>,
}

impl PartialEq for InputBlock {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid 
            && self.hexagon_uuid == other.hexagon_uuid
            && self.cluster_uuid == other.cluster_uuid
            && self.block_io == other.block_io
            && self.name == other.name
            && self.input_links == other.input_links
            && self.fill_position == other.fill_position
            && self.local_finish_counter == other.local_finish_counter
    }
}

fn init_finish_counter() -> Arc<Mutex<FinishCounter>> {
    Arc::new(Mutex::new(FinishCounter::default()))
}

impl InputBlock {
    pub fn new(name: &String, hexagon_uuid: &Uuid, cluster_uuid: &Uuid, finish_counter: &Arc<Mutex<FinishCounter>>) -> Self {
        let mut block = InputBlock {
            uuid: Uuid::new_v4(),
            hexagon_uuid: hexagon_uuid.clone(),
            cluster_uuid: cluster_uuid.clone(),

            name: name.clone(),

            block_io: BlockIoBuffer::default(),

            input_links: Vec::new(),

            fill_position: 0,

            local_finish_counter: 0,
            finish_counter: Arc::clone(finish_counter),
        };

        block.block_io.output_buffer.push(AxonSection::default());

        block
    }

    // ==================================================================================================

    pub fn apply_input(&mut self, input_ptr: &[f32], input_size: usize, offset: usize, time_length: usize) {
        // resize links, if necessary
        let maximum_size = input_size * 2 * time_length;
        if self.input_links.len() < maximum_size {
            self.input_links.resize(maximum_size as usize, UNINIT_STATE_64);
        }

        // reset potentials
        if offset == 0 {
            for section in self.block_io.output_buffer.iter_mut() {
                for axon in section.axons.iter_mut() {
                    axon.potential = 0.0f32;
                }
            }
        }

        for i in 0..input_size {
            let val = input_ptr[i];

            // update links
            let total_position = (offset + i) * 2;

            if val != 0.0f32 && self.input_links[total_position] == UNINIT_STATE_64 {
                self.input_links[total_position] = self.fill_position;
                self.input_links[total_position + 1] = self.fill_position + 1;
                self.fill_position += 2;
            }

            // resize axon-bocks of the input, if necessary
            if self.fill_position > (self.block_io.output_buffer.len() * BLOCK_DIM) as u64 {
                self.block_io.output_buffer.push(AxonSection::default());
            }

            if val != 0.0f32 && self.input_links[total_position] != UNINIT_STATE_64 {
                let neg = (val < 0.0f32) as usize;
                let target = self.input_links[total_position + neg] as usize;
                let block_pos = target / BLOCK_DIM;
                let pos_in_block = target % BLOCK_DIM;
                self.block_io.output_buffer[block_pos].axons[pos_in_block].potential = val.abs();
            }
        }
    }
}

impl Block for InputBlock {
    fn train(&mut self, _: usize, _: Arc<Mutex<dyn Block>>) {
        self.local_finish_counter = 0;
        connect_outputs(&mut self.block_io, &self.cluster_uuid, &self.hexagon_uuid, &self.uuid);
        send_forward(&self.block_io, WorkerTaskType::Train);
    }

    fn process(&mut self) {
        self.local_finish_counter = 0;
        connect_outputs(&mut self.block_io, &self.cluster_uuid, &self.hexagon_uuid, &self.uuid);
        send_forward(&self.block_io, WorkerTaskType::Process);
    }

    fn backpropagate(&mut self) {
        let mut finish_counter = self.finish_counter.lock().unwrap();
        finish_counter.counter += 1;
    }

    fn get_free_input(&mut self, _: &mut AxonSection) -> bool {
        false
    }

    fn get_uuid(&self) -> Uuid {
        self.uuid.clone()
    }

    fn get_hexagon_uud(&self) -> Uuid {
        self.hexagon_uuid.clone()
    }

    fn get_cluster_uud(&self) -> Uuid {
        self.cluster_uuid.clone()
    }

    fn get_block_io(&mut self) -> &mut BlockIoBuffer {
        return &mut self.block_io;
    }

    fn get_type(&self) -> ObjectType {
        ObjectType::InputBlock
    }

    fn set_cluster_uuid(&mut self, new_cluster_uuid: &Uuid) {
        self.cluster_uuid = new_cluster_uuid.clone();
    }

    fn serailize(&self) -> Vec<u8> {
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(&self, cfg).expect("Failed to serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_input() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));

        let name = "test-input".to_string();
        let hexagon_uuid = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();
        let mut input_block = InputBlock::new(&name, &hexagon_uuid, &cluster_uuid, &finish_counter);

        let input_values = vec![1.0, 2.0, -3.0, 4.0];
        input_block.apply_input(&input_values, input_values.len(), 2, 2);

        // check size of the resized buffers
        assert_eq!(input_block.input_links.len(), 16);
        assert_eq!(input_block.block_io.output_buffer.len(), 1);

        // check input-links
        assert_eq!(input_block.input_links[4], 0);
        assert_eq!(input_block.input_links[5], 1);
        assert_eq!(input_block.input_links[6], 2);
        assert_eq!(input_block.input_links[7], 3);
        assert_eq!(input_block.input_links[8], 4);
        assert_eq!(input_block.input_links[9], 5);
        assert_eq!(input_block.input_links[10], 6);
        assert_eq!(input_block.input_links[11], 7);

        // check axons
        assert_eq!(input_block.block_io.output_buffer[0].axons[0].potential, 1.0);
        assert_eq!(input_block.block_io.output_buffer[0].axons[1].potential, 0.0);
        assert_eq!(input_block.block_io.output_buffer[0].axons[2].potential, 2.0);
        assert_eq!(input_block.block_io.output_buffer[0].axons[3].potential, 0.0);
        assert_eq!(input_block.block_io.output_buffer[0].axons[4].potential, 0.0);
        assert_eq!(input_block.block_io.output_buffer[0].axons[5].potential, 3.0);
        assert_eq!(input_block.block_io.output_buffer[0].axons[6].potential, 4.0);
        assert_eq!(input_block.block_io.output_buffer[0].axons[7].potential, 0.0);
    }

    #[test]
    fn test_serialize_deserialize() {
        let original = InputBlock::default();

        let cfg = bincode::config::standard();
        let serialized: Vec<u8> = bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: InputBlock = bincode::serde::decode_from_slice(&serialized, cfg).expect("Failed to deserialize").0;
        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
