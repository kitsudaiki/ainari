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
use rand::Rng;
use uuid::Uuid;
use std::time::Instant;

use hanami_common::constants::*;

use crate::core::processing::output_buffer::*;
use crate::core::cluster_handler::*;

use super::axons::*;
use super::block_trait::*;
use super::block_io::*;

// ==================================================================================================

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

pub struct OutputBlock {
    pub uuid: Uuid,
    pub hexagon_uuid: Uuid,
    pub cluster_uuid: Uuid,

    pub block_io: BlockIoBuffer,

    pub weights: Vec<f32>,
    pub block_outputs: Vec<OutputNeuron>,

    pub output_buffer_name: String,
    pub output_buffer: Option<Arc<Mutex<OutputBuffer>>>,
}

impl OutputBlock {
    pub fn new(hexagon_uuid: &Uuid, cluster_uuid: &Uuid, output_buffer_name: &String) -> Self {
        let mut block = OutputBlock {
            uuid: Uuid::new_v4(),
            hexagon_uuid: hexagon_uuid.clone(),
            cluster_uuid: cluster_uuid.clone(),

            block_io: BlockIoBuffer::default(),

            weights: Vec::new(),
            block_outputs: Vec::new(),

            output_buffer_name: output_buffer_name.clone(),
            output_buffer: None,
        };

        block.block_io.input_buffer.push(AxonSection::default());
        block.block_io.inputs_in_use = 0;

        block
    }

    fn connect_output_buffer(&mut self) {
        // connect output-buffer if not already done
        if self.output_buffer.is_none() {
            let root_handler = CLUSTER_HANDLER.read().unwrap();
            if let Some(output_buffer_mutex) = root_handler.get_output_buffer(&self.cluster_uuid, &self.output_buffer_name) {
                self.output_buffer = Some(output_buffer_mutex.clone());
                let mut output_buffer = output_buffer_mutex.lock().unwrap();
                output_buffer.number_of_connected_blocks += 1;
            }
        }
    }

    fn process_block(&mut self) {
        // reset output-values
        for output_neuron in self.block_outputs.iter_mut() {
            output_neuron.output_value = 0.0f32;
        }

        let input_buffer = &mut self.block_io.input_buffer[0];
        // calculate block-internal output
        for (x, axon) in input_buffer.axons.iter_mut().enumerate() {
            if axon.potential == 0.0f32 {
                continue;
            }

            axon.potential = 1.0f32 / (1.0f32 + (-1.0f32 * axon.potential).exp());
            for (y, output_neuron) in self.block_outputs.iter_mut().enumerate() {
                output_neuron.output_value += self.weights[(y * BLOCK_DIM) + x] * axon.potential;
            }
        }
    }
}

// ==================================================================================================

impl Block for OutputBlock {
    fn train(&mut self, _: usize, own: Arc<Mutex<dyn Block>>) {
        self.connect_output_buffer();

        // resize output and wights and get expected values from output-buffer
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let mut rng = rand::rng();

            let output_buffer = output_buffer_mutex.lock().unwrap();
            self.block_outputs.resize_with(output_buffer.output_neurons.len(), OutputNeuron::default);
            let number_fo_weights = self.block_outputs.len() * BLOCK_DIM;
            self.weights.resize_with(number_fo_weights, || rng.random_range(-0.5..0.5));
        } else {
            // TODO: error handling
        }

        self.process_block();

        // process output-buffer
        let mut already_done = false;
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let mut output_buffer = output_buffer_mutex.lock().unwrap();
            for (i, local_neuron) in self.block_outputs.iter().enumerate() {
                output_buffer.output_neurons[i].output_value += local_neuron.output_value;
            }

            if output_buffer.already_finalized == false  {
                output_buffer.local_finish_counter += 1;
                if output_buffer.local_finish_counter == output_buffer.number_of_connected_blocks 
                {
                    output_buffer.finalize();
                    output_buffer.backpropagate();
                    already_done = true;
                } else {
                    output_buffer.unfinished_blocks.push(own);
                }
            }
            else {
                already_done = true;
            }
        }

        if already_done {
            self.backpropagate();
        }
    }

    fn process(&mut self) {
        self.process_block();

        // process output-buffer
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let mut output_buffer = output_buffer_mutex.lock().unwrap();
            for (i, local_neuron) in self.block_outputs.iter().enumerate() {
                output_buffer.output_neurons[i].output_value += local_neuron.output_value;
            }
            output_buffer.local_finish_counter += 1;

            if output_buffer.local_finish_counter >= output_buffer.number_of_connected_blocks {
                output_buffer.finalize();
            }
        }
    }

    fn backpropagate(&mut self) {
        self.connect_output_buffer();
    
        // resize output and wights and get expected values from output-buffer
        if let Some(output_buffer_mutex) = &self.output_buffer {
            let output_buffer = output_buffer_mutex.lock().unwrap();
            self.block_outputs.resize_with(output_buffer.output_neurons.len(), OutputNeuron::default);
            for i in 0..self.block_outputs.len() {
                self.block_outputs[i].expected_value = output_buffer.output_neurons[i].expected_value;
            }
        } else {
            // TODO: error
        }

        // backpropagate block
        let input_buffer = &mut self.block_io.input_buffer[0];
        let learn_value = 0.1f32;
        for (x, axon) in input_buffer.axons.iter_mut().enumerate() {
            axon.delta = 0.0f32;
            if axon.potential == 0.0f32 {
                continue;
            }

            for (y, output_neuron) in self.block_outputs.iter_mut().enumerate() {
                let weight = &mut self.weights[(y * BLOCK_DIM) + x];
                let update = output_neuron.expected_value;
                axon.delta += update * (*weight);
                *weight -= update * learn_value * axon.potential;
            }

            axon.delta *= axon.potential * (1.0f32 - axon.potential);
        }

        send_backward(&self.block_io);
    }

    fn get_free_input(&mut self, axon_section: &mut AxonSection) -> bool {
        if self.block_io.inputs_in_use == 0 {
            axon_section.target_block_uuid = self.uuid.clone();
            axon_section.target_hexagon_uuid = self.hexagon_uuid.clone();
            axon_section.target_pos = 0;
            self.block_io.input_buffer[0] = axon_section.clone();
            self.block_io.inputs_in_use = 1;
            return true;
        }

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
}
