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

use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ainari_cluster_parser::cluster_meta_structs::Settings;
use ainari_common::constants::*;
use ainari_common::enums::*;
use ainari_common::error::AinariError;
use ainari_common::functions::*;

use crate::core::processing::finish_counter::FinishCounter;

use super::axons::*;
use super::block_io::*;
use super::block_trait::*;

use super::super::processing::worker_queue::*;

// ==================================================================================================

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
pub struct Synapse {
    pub border: f32,
    pub weight_1: f32,
    pub weight_2: f32,

    pub active_counter: u8,
    pub target_neuron_id: u16,
}

impl Synapse {
    pub fn default() -> Self {
        Synapse {
            weight_1: 0.0f32,
            weight_2: 0.0f32,

            border: 1.0f32,
            active_counter: 0,
            target_neuron_id: UNINIT_STATE_16,
        }
    }
}

// ==================================================================================================

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SynapseSection {
    #[serde(with = "BigArray")]
    pub synapses: [Synapse; BLOCK_DIM],
}

impl SynapseSection {
    pub fn default() -> Self {
        let mut section = SynapseSection {
            synapses: std::array::from_fn(|_| Synapse::default()),
        };

        section.synapses[0].target_neuron_id = rand::rng().random_range(0..BLOCK_DIM) as u16;
        section.synapses[0].weight_1 = rand::rng().random_range(-0.5f32..0.5f32);
        section.synapses[0].weight_2 = rand::rng().random_range(-0.5f32..0.5f32);

        section
    }
}

// ==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Connection {
    pub lower_bound: f32,
    pub source_input: u16,
    pub next: u16,
    pub used: bool,
}

impl Connection {
    pub fn default() -> Self {
        Connection {
            lower_bound: 0.0f32,
            source_input: UNINIT_STATE_16,
            next: UNINIT_STATE_16,
            used: false,
        }
    }
}

// ==================================================================================================

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Neuron {
    pub input: f32,
    pub refractory_time: u32,
}

impl Neuron {
    pub fn default() -> Self {
        Neuron {
            input: 0.0f32,
            refractory_time: 0,
        }
    }
}

// ==================================================================================================

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct CoreBlock {
    pub uuid: Uuid,
    pub hexagon_uuid: Uuid,
    pub cluster_uuid: Uuid,

    pub block_io: BlockIoBuffer,
    cluster_settings: Settings,

    // HINT (kitsudaiki): this has to be a Box instead of a static array to avoid a stack-overflow, because the object is too big
    pub synapse_sections: Box<[SynapseSection]>,
    #[serde(with = "BigArray")]
    pub neurons: [Neuron; BLOCK_DIM * 3],
    #[serde(with = "BigArray")]
    connections: [Connection; BLOCK_DIM * 6],

    pub section_counter: u64,
}

impl CoreBlock {
    pub fn new(hexagon_uuid: &Uuid, cluster_uuid: &Uuid, cluster_settings: &Settings) -> Self {
        // internal visilization of the blocks:
        //
        // +---+ +---+ +---+
        // | 1 | | 3 | | 5 |
        // +---+ +---+ +---+
        // | 0 | | 2 | | 4 |
        // +---+ +---+ +---+
        //
        let capacity = BLOCK_DIM * 2 * 3;
        let mut vec = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            vec.push(SynapseSection::default());
        }

        let mut block = CoreBlock {
            uuid: Uuid::new_v4(),
            hexagon_uuid: *hexagon_uuid,
            cluster_uuid: *cluster_uuid,

            block_io: BlockIoBuffer::default(),
            cluster_settings: cluster_settings.clone(),

            synapse_sections: vec.into_boxed_slice(),
            neurons: std::array::from_fn(|_| Neuron::default()),
            connections: std::array::from_fn(|_| Connection::default()),

            // pre-initialized number of synapses, one for each possible input-axon
            section_counter: 0,
        };

        block.block_io.output_buffer.push(AxonSection::default());
        block.block_io.input_buffer.push(AxonSection::default());
        block.block_io.input_buffer.push(AxonSection::default());
        block.block_io.inputs_in_use = 0;

        for i in 0..(2 * BLOCK_DIM) {
            block.connections[i].lower_bound = 0.0f32;
            block.connections[i].source_input = i as u16;
        }

        block
    }

    fn check_and_resize_block(&mut self) {
        // resize block in case it is filled enough
        if self.block_io.output_buffer.len() == 1
            && self.section_counter as f32 >= ((2 * BLOCK_DIM) as f32 * 0.9f32)
        {
            self.block_io.output_buffer.push(AxonSection::default());
        }
        if self.block_io.output_buffer.len() == 2
            && self.section_counter as f32 >= ((4 * BLOCK_DIM) as f32 * 0.9f32)
        {
            self.block_io.output_buffer.push(AxonSection::default());
        }
    }

    fn apply_output(&mut self) {
        let mut counter = 0;
        let mut neuron;

        for buffer in self.block_io.output_buffer.iter_mut() {
            for axon in buffer.data.axons.iter_mut() {
                neuron = &mut self.neurons[counter];

                axon.potential /= self.cluster_settings.neuron_cooldown;
                neuron.refractory_time >>= 1;

                if neuron.refractory_time == 0 {
                    neuron.refractory_time = self.cluster_settings.refractory_time;
                }

                // // experimental stuff
                // axon.potential = neuron.input.signum() * (neuron.input.abs().log2() + 1.0f32);
                axon.potential = neuron.input;

                neuron.input = 0.0f32;
                axon.delta = 0.0f32;

                counter += 1;
            }
        }
    }
}

// ==================================================================================================

#[inline]
fn create_new_synapse(
    synapse: &mut Synapse,
    remaining_weight: f32,
    number_of_output_blocks: usize,
    random_seed: &mut u32,
) {
    let rand_max = RAND_MAX as f32;
    let sig_neg = 0.5f32;
    let mut sign_rand;

    synapse.weight_1 = ((pcg_hash(random_seed) as f32) / rand_max) / 10.0f32;
    sign_rand = (pcg_hash(random_seed) % 1000) as f32;
    synapse.weight_1 *= 1.0f32 - (1000.0f32 * sig_neg > sign_rand) as u8 as f32 * 2.0f32;

    synapse.weight_2 = ((pcg_hash(random_seed) as f32) / rand_max) / 10.0f32;
    sign_rand = (pcg_hash(random_seed) % 1000) as f32;
    synapse.weight_2 *= 1.0f32 - (1000.0f32 * sig_neg > sign_rand) as u8 as f32 * 2.0f32;

    synapse.border = remaining_weight;
    synapse.active_counter = 50;
    synapse.target_neuron_id =
        (pcg_hash(random_seed) % (number_of_output_blocks * BLOCK_DIM) as u32) as u16
}

#[inline]
fn search_free_connection(connections: &[Connection; BLOCK_DIM * 6]) -> usize {
    for (i, conn) in connections.iter().enumerate() {
        if conn.source_input == UNINIT_STATE_16 {
            return i;
        }
    }

    UNINIT_STATE_16 as usize
}

#[inline]
fn train_section(
    section: &mut SynapseSection,
    connection: &Connection,
    neurons: &mut [Neuron; BLOCK_DIM * 3],
    axon: &Axon,
    number_of_output_blocks: usize,
    random_seed: &mut u32,
    connections: &mut [Connection; BLOCK_DIM * 6],
) -> bool {
    let mut ratio;
    let mut potential = axon.potential - connection.lower_bound;
    let mut condition;
    let mut prev_border = 0.0f32;
    let mut target_neuron;

    // iterate over all synapses in the section
    for (pos, synapse) in section.synapses.iter_mut().enumerate() {
        if potential <= POTENTIAL_BORDER {
            break;
        }

        // create new synapse if necesarry and training is active
        if synapse.target_neuron_id == UNINIT_STATE_16 {
            // because of the initialize of the section, the first position should
            // always be filled
            assert!(pos > 0);
            let remaining_weight = prev_border * 2.0f32;
            create_new_synapse(
                synapse,
                remaining_weight,
                number_of_output_blocks,
                random_seed,
            );
        }

        if potential < synapse.border {
            condition = potential < (1.0f32 - RELATIVE_CREATE_BORDER) * synapse.border
                && potential > RELATIVE_CREATE_BORDER * synapse.border
                && potential < synapse.border - ABSOLUTE_CREATE_BORDER
                && potential > ABSOLUTE_CREATE_BORDER;

            synapse.border = synapse.border * (!condition) as u8 as f32
                + (synapse.border / 2.0f32) * (condition) as u8 as f32;
        }

        prev_border = synapse.border;

        ratio = 1.0f32;
        if potential < synapse.border {
            ratio = (1.0f32 / synapse.border) * potential;
        }

        target_neuron = &mut neurons
            [(synapse.target_neuron_id % (number_of_output_blocks * BLOCK_DIM) as u16) as usize];
        target_neuron.input += synapse.weight_1 * ratio * (potential > synapse.border) as u8 as f32;

        target_neuron = &mut neurons[((synapse.target_neuron_id + 1)
            % (number_of_output_blocks * BLOCK_DIM) as u16)
            as usize];
        target_neuron.input += synapse.weight_2 * ratio * (potential > synapse.border) as u8 as f32;

        // update loop-counter
        potential -= synapse.border;
    }

    if potential > POTENTIAL_BORDER {
        if connection.next == UNINIT_STATE_16 {
            return true;
        }

        if connection.next != UNINIT_STATE_16 {
            let next_connection = &mut connections[connection.next as usize];
            if next_connection.lower_bound < potential {
                next_connection.lower_bound = potential;
            }
        }
    }

    false
}

#[inline]
fn process_section(
    section: &mut SynapseSection,
    connection: &Connection,
    neurons: &mut [Neuron; BLOCK_DIM * 3],
    axon: &Axon,
    number_of_output_blocks: usize,
) {
    let mut ratio;
    let mut potential = axon.potential - connection.lower_bound;
    let mut target_neuron;

    // iterate over all synapses in the section
    for synapse in section.synapses.iter_mut() {
        if potential <= POTENTIAL_BORDER {
            break;
        }

        if synapse.target_neuron_id == UNINIT_STATE_16 {
            break;
        }

        ratio = 1.0f32;
        if potential < synapse.border {
            ratio = (1.0f32 / synapse.border) * potential;
        }

        target_neuron = &mut neurons
            [(synapse.target_neuron_id % (number_of_output_blocks * BLOCK_DIM) as u16) as usize];
        target_neuron.input += synapse.weight_1 * ratio * (potential > synapse.border) as u8 as f32;

        target_neuron = &mut neurons[((synapse.target_neuron_id + 1)
            % (number_of_output_blocks * BLOCK_DIM) as u16)
            as usize];
        target_neuron.input += synapse.weight_2 * ratio * (potential > synapse.border) as u8 as f32;

        // update loop-counter
        potential -= synapse.border;
    }
}

#[inline]
fn backpropagate_section(
    section: &mut SynapseSection,
    connection: &mut Connection,
    source_axon: &mut Axon,
    output_buffer: &[AxonSection],
) {
    let mut potential = source_axon.potential - connection.lower_bound;
    let mut delta;
    let mut target_axon;
    let mut output_block_id;
    let mut output_axon_id;

    // iterate over all synapses in the section
    for synapse in section.synapses.iter_mut() {
        if potential <= POTENTIAL_BORDER {
            break;
        }

        if synapse.target_neuron_id == UNINIT_STATE_16 {
            break;
        }

        output_block_id =
            ((synapse.target_neuron_id / BLOCK_DIM as u16) as usize) % output_buffer.len();
        output_axon_id = (synapse.target_neuron_id % BLOCK_DIM as u16) as usize;
        target_axon = &output_buffer[output_block_id].data.axons[output_axon_id];
        delta = target_axon.delta * synapse.weight_1;
        synapse.weight_1 -= CORE_TRAIN_VALUE * target_axon.delta;
        source_axon.delta += delta;

        output_block_id =
            (((synapse.target_neuron_id + 1) / BLOCK_DIM as u16) as usize) % output_buffer.len();
        output_axon_id = ((synapse.target_neuron_id + 1) % BLOCK_DIM as u16) as usize;
        target_axon = &output_buffer[output_block_id].data.axons[output_axon_id];
        delta = target_axon.delta * synapse.weight_2;
        synapse.weight_2 -= CORE_TRAIN_VALUE * target_axon.delta;
        source_axon.delta += delta;

        potential -= synapse.border;
    }
}

// ==================================================================================================

impl Block for CoreBlock {
    fn train(
        &mut self,
        _: usize,
        _: Arc<Mutex<dyn Block>>,
        _: u64,
    ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.check_and_resize_block();
        let number_of_output_blocks = self.block_io.output_buffer.len();
        let mut random_seed = rand::rng().random_range(1..(RAND_MAX - 1)) as u32;

        for i in 0..(6 * BLOCK_DIM) {
            let conn = self.connections[i];
            if conn.source_input == UNINIT_STATE_16 {
                continue;
            }

            let input_block_id = (conn.source_input / BLOCK_DIM as u16) as usize;
            let axon_id = (conn.source_input % BLOCK_DIM as u16) as usize;
            let axon = &self.block_io.input_buffer[input_block_id].data.axons[axon_id];
            if axon.potential != 0.0f32 {
                if !conn.used {
                    self.section_counter += 1;
                    let temp_conn = &mut self.connections[i];
                    temp_conn.used = true;
                }
                let section = &mut self.synapse_sections[i];
                let need_next = train_section(
                    section,
                    &conn,
                    &mut self.neurons,
                    axon,
                    number_of_output_blocks,
                    &mut random_seed,
                    &mut self.connections,
                );
                if need_next {
                    let next = search_free_connection(&self.connections) as u16;
                    let mut temp_conn = &mut self.connections[i];
                    temp_conn.next = next;
                    if next != UNINIT_STATE_16 {
                        temp_conn = &mut self.connections[next as usize];
                        temp_conn.source_input = conn.source_input;
                    }
                }
            }
        }

        for axon_block in self.block_io.output_buffer.iter_mut() {
            for axon in axon_block.data.axons.iter_mut() {
                axon.delta = 0.0f32;
            }
        }

        self.apply_output();

        Ok(None)
    }

    fn process(&mut self, _: u64) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.check_and_resize_block();
        let number_of_output_blocks = self.block_io.output_buffer.len();

        for (i, conn) in self.connections.iter().enumerate() {
            if conn.source_input == UNINIT_STATE_16 {
                continue;
            }

            let input_block_id = (conn.source_input / BLOCK_DIM as u16) as usize;
            let axon_id = (conn.source_input % BLOCK_DIM as u16) as usize;
            let axon = &self.block_io.input_buffer[input_block_id].data.axons[axon_id];
            if axon.potential != 0.0f32 {
                if !conn.used {
                    continue;
                }
                let section = &mut self.synapse_sections[i];
                process_section(
                    section,
                    conn,
                    &mut self.neurons,
                    axon,
                    number_of_output_blocks,
                );
            }
        }

        self.apply_output();

        Ok(None)
    }

    fn backpropagate(&mut self, _: u64) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        // // experimental stuff
        // for axon_section in self.block_io.input_buffer.iter_mut() {
        //     for axon in axon_section.axons.iter_mut() {
        //         axon.delta *= 1.4427f32 * (0.5f32).powf(axon.potential);
        //     }
        // }

        for (i, conn) in self.connections.iter_mut().enumerate() {
            if conn.source_input == UNINIT_STATE_16 {
                continue;
            }

            let input_block_id = (conn.source_input / BLOCK_DIM as u16) as usize;
            let axon_id = (conn.source_input % BLOCK_DIM as u16) as usize;
            let source_axon = &mut self.block_io.input_buffer[input_block_id].data.axons[axon_id];
            if source_axon.potential > 0.0f32 {
                let section = &mut self.synapse_sections[i];
                backpropagate_section(section, conn, source_axon, &self.block_io.output_buffer);
            }
        }

        Ok(None)
    }

    fn finalize_train(&mut self, cycle_number: u64) -> Result<(), AinariError> {
        connect_outputs(
            &mut self.block_io,
            &self.cluster_uuid,
            &self.hexagon_uuid,
            &self.uuid,
        )?;
        send_forward(&self.block_io, WorkerTaskType::Train, cycle_number);

        Ok(())
    }

    fn finalize_process(&mut self, cycle_number: u64) -> Result<(), AinariError> {
        connect_outputs(
            &mut self.block_io,
            &self.cluster_uuid,
            &self.hexagon_uuid,
            &self.uuid,
        )?;
        send_forward(&self.block_io, WorkerTaskType::Process, cycle_number);

        Ok(())
    }

    fn finalize_backpropagate(&mut self, cycle_number: u64) -> Result<bool, AinariError> {
        let ret = send_backward_with_retry(&mut self.block_io, cycle_number);

        Ok(ret)
    }

    fn get_free_input(&mut self, axon_section: &mut AxonSection) -> bool {
        if self.block_io.inputs_in_use == 0 {
            axon_section.target_block_uuid = self.uuid;
            axon_section.target_hexagon_uuid = self.hexagon_uuid;
            axon_section.target_pos = 0;
            self.block_io.input_buffer[0] = axon_section.clone();
            self.block_io.inputs_in_use = 1;
            return true;
        }

        if self.block_io.inputs_in_use == 1 {
            axon_section.target_block_uuid = self.uuid;
            axon_section.target_hexagon_uuid = self.hexagon_uuid;
            axon_section.target_pos = 1;
            self.block_io.input_buffer[1] = axon_section.clone();
            self.block_io.inputs_in_use = 2;
            return true;
        }

        false
    }

    fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    fn get_hexagon_uud(&self) -> Uuid {
        self.hexagon_uuid
    }
    fn get_cluster_uud(&self) -> Uuid {
        self.cluster_uuid
    }

    fn get_block_io(&mut self) -> &mut BlockIoBuffer {
        &mut self.block_io
    }

    fn get_type(&self) -> ObjectType {
        ObjectType::CoreBlock
    }

    fn set_cluster_uuid(&mut self, new_cluster_uuid: &Uuid) {
        self.cluster_uuid = *new_cluster_uuid;
    }

    fn serailize(&self) -> Vec<u8> {
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(self, cfg).expect("Failed to serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let settings = Settings::default();
        let original = CoreBlock::new(&Uuid::new_v4(), &Uuid::new_v4(), &settings);

        let cfg = bincode::config::standard().with_variable_int_encoding();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: CoreBlock = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;

        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
