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
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ainari_common::constants::*;
use ainari_common::enums::*;
use ainari_common::error::AinariError;

use crate::core::cluster_handler::*;
use crate::core::processing::finish_counter::FinishCounter;
use crate::core::processing::output_buffer::*;

use super::axons::*;
use super::block_io::*;
use super::block_trait::*;

// ==================================================================================================

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputNeuron {
    pub output_value: f32,
    pub expected_value: f32,
}

impl OutputNeuron {
    pub fn default() -> Self {
        OutputNeuron {
            output_value: 0.0f32,
            expected_value: 0.0f32,
        }
    }
}

// ==================================================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputBlock {
    pub uuid: Uuid,
    pub hexagon_uuid: Uuid,
    pub cluster_uuid: Uuid,

    pub block_io: BlockIoBuffer,

    pub weights: Vec<f32>,
    pub block_outputs: Vec<OutputNeuron>,

    pub output_buffer_name: String,
    pub was_already_connected: bool,

    #[serde(skip)]
    pub output_buffer: Option<Arc<Mutex<OutputBuffer>>>,
}

impl PartialEq for OutputBlock {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
            && self.hexagon_uuid == other.hexagon_uuid
            && self.cluster_uuid == other.cluster_uuid
            && self.block_io == other.block_io
            && self.weights == other.weights
            && self.block_outputs == other.block_outputs
            && self.output_buffer_name == other.output_buffer_name
            && self.was_already_connected == other.was_already_connected
    }
}

impl OutputBlock {
    pub fn new(hexagon_uuid: &Uuid, cluster_uuid: &Uuid, output_buffer_name: &str) -> Self {
        let mut block = OutputBlock {
            uuid: Uuid::new_v4(),
            hexagon_uuid: *hexagon_uuid,
            cluster_uuid: *cluster_uuid,

            block_io: BlockIoBuffer::default(),

            weights: Vec::new(),
            block_outputs: Vec::new(),

            output_buffer_name: output_buffer_name.to_owned(),
            was_already_connected: false,

            output_buffer: None,
        };

        block.block_io.input_buffer.push(AxonSection::default());
        block.block_io.inputs_in_use = 0;

        block
    }

    fn connect_output_buffer(&mut self) -> Result<(), AinariError> {
        // connect output-buffer if not already done
        if self.output_buffer.is_none() {
            let root_handler = CLUSTER_HANDLER.read().unwrap();
            let output_buffer_mutex =
                root_handler.get_output_buffer(&self.cluster_uuid, &self.output_buffer_name)?;

            self.output_buffer = Some(output_buffer_mutex.clone());
            let mut output_buffer = output_buffer_mutex.lock().unwrap();
            // after a checkpoint-restore the block must be connected to the buffer again,
            // but is not allowed to increase the counter further
            if !self.was_already_connected {
                output_buffer.number_of_connected_blocks += 1;
            }
            self.was_already_connected = true;
        }

        Ok(())
    }

    fn process_block(&mut self) {
        // reset output-values
        for output_neuron in self.block_outputs.iter_mut() {
            output_neuron.output_value = 0.0f32;
        }

        let input_buffer = &mut self.block_io.input_buffer[0];
        // calculate block-internal output
        for (x, axon) in input_buffer.data.axons.iter_mut().enumerate() {
            if axon.potential == 0.0f32 {
                continue;
            }

            axon.potential = 1.0f32 / (1.0f32 + (-axon.potential).exp());
            for (y, output_neuron) in self.block_outputs.iter_mut().enumerate() {
                output_neuron.output_value += self.weights[(y * BLOCK_DIM) + x] * axon.potential;
            }
        }
    }
}

// ==================================================================================================

impl Block for OutputBlock {
    fn train(
        &mut self,
        _: usize,
        own: Arc<Mutex<dyn Block>>,
        cycle_number: u64,
    ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.connect_output_buffer()?;

        // resize output and wights and get expected values from output-buffer
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let mut rng = rand::rng();

            let output_buffer = output_buffer_mutex.lock().unwrap();
            self.block_outputs
                .resize_with(output_buffer.output_neurons.len(), OutputNeuron::default);
            let number_fo_weights = self.block_outputs.len() * BLOCK_DIM;
            self.weights
                .resize_with(number_fo_weights, || rng.random_range(-0.5..0.5));
        } else {
            // TODO: error handling
        }

        self.process_block();

        // process output-buffer
        let mut finish_counter_option = None;
        let mut already_done = false;
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let mut output_buffer = output_buffer_mutex.lock().unwrap();
            for (i, local_neuron) in self.block_outputs.iter().enumerate() {
                output_buffer.output_neurons[i].output_value += local_neuron.output_value;
            }

            if !output_buffer.already_finalized {
                if output_buffer.update_finish_counter(cycle_number) {
                    output_buffer.finalize_train();
                    output_buffer.backpropagate(cycle_number);
                    finish_counter_option = Some(output_buffer.finish_counter_mutex.clone());
                    already_done = true;
                } else {
                    output_buffer.unfinished_blocks.push(own);
                }
            } else {
                already_done = true;
            }
        }

        if already_done {
            self.backpropagate(cycle_number)?;
        }

        Ok(finish_counter_option)
    }

    fn process(
        &mut self,
        cycle_number: u64,
    ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.connect_output_buffer()?;
        self.process_block();

        let mut finish_counter_option = None;

        // process output-buffer
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let mut output_buffer = output_buffer_mutex.lock().unwrap();
            for (i, local_neuron) in self.block_outputs.iter().enumerate() {
                output_buffer.output_neurons[i].output_value += local_neuron.output_value;
            }

            if output_buffer.update_finish_counter(cycle_number) {
                output_buffer.finalize_processing();
                finish_counter_option = Some(output_buffer.finish_counter_mutex.clone());
            }
        }

        Ok(finish_counter_option)
    }

    fn backpropagate(
        &mut self,
        cycle_number: u64,
    ) -> Result<Option<Arc<Mutex<FinishCounter>>>, AinariError> {
        self.connect_output_buffer()?;

        // resize output and wights and get expected values from output-buffer
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let output_buffer = output_buffer_mutex.lock().unwrap();
            self.block_outputs
                .resize_with(output_buffer.output_neurons.len(), OutputNeuron::default);
            for i in 0..self.block_outputs.len() {
                self.block_outputs[i].expected_value =
                    output_buffer.output_neurons[i].expected_value;
            }
        } else {
            // TODO: error
        }

        // backpropagate block
        let input_buffer = &mut self.block_io.input_buffer[0];
        for (x, axon) in input_buffer.data.axons.iter_mut().enumerate() {
            axon.delta = 0.0f32;
            if axon.potential == 0.0f32 {
                continue;
            }

            for (y, output_neuron) in self.block_outputs.iter_mut().enumerate() {
                let weight = &mut self.weights[(y * BLOCK_DIM) + x];
                let update = output_neuron.expected_value;
                axon.delta += update * (*weight);
                *weight -= update * OUTPUT_TRAIN_VALUE * axon.potential;
            }

            axon.delta *= axon.potential * (1.0f32 - axon.potential);
        }

        send_backward(&mut self.block_io, cycle_number);

        Ok(None)
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

        false
    }

    fn finalize_train(&mut self, _: u64) -> Result<(), AinariError> {
        Ok(())
    }

    fn finalize_process(&mut self, _: u64) -> Result<(), AinariError> {
        Ok(())
    }

    fn finalize_backpropagate(&mut self, _: u64) -> Result<bool, AinariError> {
        Ok(true)
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
        ObjectType::OutputBlock
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
        let original = OutputBlock::new(&Uuid::new_v4(), &Uuid::new_v4(), "test");

        let cfg = bincode::config::standard();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: OutputBlock = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;
        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
