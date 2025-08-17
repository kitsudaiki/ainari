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

use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ainari_common::enums::*;

use crate::core::processing::finish_counter::FinishCounter;
use crate::core::processing::worker_queue::*;

use super::super::blocks::block_trait::*;
use super::super::blocks::output_block::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputBuffer {
    #[allow(dead_code)]
    pub uuid: Uuid,
    #[allow(dead_code)]
    pub hexagon_uuid: Uuid,
    pub cluster_uuid: Uuid,
    pub name: String,

    pub output_neurons: Vec<OutputNeuron>,
    pub output_type: OutputType,
    pub output_size: u64,

    pub already_finalized: bool,
    pub number_of_connected_blocks: u64,
    pub local_finish_counter: u64,
    #[serde(skip, default = "init_finish_counter")]
    pub finish_counter_mutex: Arc<Mutex<FinishCounter>>,
    #[serde(skip, default = "init_unfinished_blocks")]
    pub unfinished_blocks: Vec<Arc<Mutex<dyn Block>>>,
}

impl PartialEq for OutputBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
            && self.hexagon_uuid == other.hexagon_uuid
            && self.cluster_uuid == other.cluster_uuid
            && self.name == other.name
            && self.output_neurons == other.output_neurons
            && self.output_type == other.output_type
            && self.output_size == other.output_size
            && self.already_finalized == other.already_finalized
            && self.number_of_connected_blocks == other.number_of_connected_blocks
            && self.local_finish_counter == other.local_finish_counter
    }
}

fn init_finish_counter() -> Arc<Mutex<FinishCounter>> {
    Arc::new(Mutex::new(FinishCounter::default()))
}

fn init_unfinished_blocks() -> Vec<Arc<Mutex<dyn Block>>> {
    Vec::new()
}

impl OutputBuffer {
    pub fn new(
        name: &str,
        hexagon_uuid: &Uuid,
        cluster_uuid: &Uuid,
        output_type: &OutputType,
        finish_counter_mutex: &Arc<Mutex<FinishCounter>>,
    ) -> Self {
        OutputBuffer {
            uuid: *hexagon_uuid,
            hexagon_uuid: *hexagon_uuid,
            cluster_uuid: *cluster_uuid,
            name: name.to_owned(),

            output_neurons: Vec::new(),
            output_type: output_type.clone(),
            output_size: 0,

            already_finalized: false,
            number_of_connected_blocks: 0,
            local_finish_counter: 0,
            finish_counter_mutex: Arc::clone(finish_counter_mutex),
            unfinished_blocks: Vec::new(),
        }
    }

    pub fn update_buffer(&mut self, number_of_outputs: usize) {
        let mut number_of_outputs_copy = number_of_outputs;

        if self.output_size < number_of_outputs_copy as u64 {
            self.output_size = number_of_outputs_copy as u64;

            if self.output_type == OutputType::FloatOutput {
                number_of_outputs_copy *= 32;
            }
            if self.output_type == OutputType::IntOutput {
                number_of_outputs_copy *= 64;
            }

            self.output_neurons
                .resize_with(number_of_outputs_copy, OutputNeuron::default);
        }
    }

    pub fn finalize_train(&mut self) {
        for out in self.output_neurons.iter_mut() {
            if out.output_value != 0.0f32 {
                out.output_value = 1.0f32 / (1.0f32 + (-out.output_value).exp());
            }
        }

        self.already_finalized = true;
    }

    pub fn finalize_processing(&mut self) {
        for out in self.output_neurons.iter_mut() {
            if out.output_value != 0.0f32 {
                out.output_value = 1.0f32 / (1.0f32 + (-out.output_value).exp());
            }
        }

        self.already_finalized = true;
        self.unfinished_blocks.clear();
    }

    pub fn backpropagate(&mut self, cycle_number: u64) {
        for out in self.output_neurons.iter_mut() {
            let delta = out.output_value - out.expected_value;
            out.expected_value = delta * out.output_value * (1.0f32 - out.output_value);
        }

        let mut worker_queue = WORKER_QUEUE.lock().unwrap();
        for block in self.unfinished_blocks.iter() {
            let worker_task = WorkerTask {
                task_type: WorkerTaskType::Backpropagate,
                block: Arc::clone(block),
                cycle_number,
            };

            worker_queue.add(worker_task);
        }
        self.unfinished_blocks.clear();
    }

    pub fn reset_output(&mut self) {
        for out in self.output_neurons.iter_mut() {
            out.output_value = 0.0f32;
        }
        self.local_finish_counter = 0;
    }

    pub fn serailize(&self) -> Vec<u8> {
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(self, cfg).expect("Failed to serialize")
    }

    pub fn update_finish_counter(&mut self, cycle_number: u64) -> bool {
        let finish_counter = self.finish_counter_mutex.lock().unwrap();
        let expected_cycle_number = finish_counter.get_expected_cycle_number();
        if cycle_number == expected_cycle_number {
            self.local_finish_counter += 1;
            if self.local_finish_counter >= self.number_of_connected_blocks {
                return true;
            }
        }

        false
    }
}

pub fn convert_output_to_buffer(buffer: &mut [f32], output_buffer: &mut OutputBuffer) -> usize {
    output_buffer.already_finalized = false;
    match output_buffer.output_type {
        OutputType::PlainOutput => handle_plain_output(buffer, output_buffer),
        OutputType::BoolOutput => handle_bool_output(buffer, output_buffer),
        OutputType::IntOutput => handle_int_output(buffer, output_buffer),
        OutputType::FloatOutput => handle_float_output(buffer, output_buffer),
    }
}

pub fn convert_buffer_to_expected(
    output_buffer: &mut OutputBuffer,
    buffer: &[f32],
    buffer_size: u64,
) -> u64 {
    output_buffer.update_buffer(buffer.len());
    output_buffer.already_finalized = false;
    match output_buffer.output_type {
        OutputType::PlainOutput => handle_plain_expected(output_buffer, buffer, buffer_size),
        OutputType::BoolOutput => handle_bool_expected(output_buffer, buffer, buffer_size),
        OutputType::IntOutput => handle_int_expected(output_buffer, buffer, buffer_size),
        OutputType::FloatOutput => handle_float_expected(output_buffer, buffer, buffer_size),
    }
}

fn handle_plain_output(buffer: &mut [f32], output_buffer: &OutputBuffer) -> usize {
    let number_of_outputs = min(buffer.len(), output_buffer.output_neurons.len());

    for (i, buffer) in buffer.iter_mut().enumerate().take(number_of_outputs) {
        *buffer = output_buffer.output_neurons[i].output_value;
    }

    number_of_outputs
}

fn handle_bool_output(buffer: &mut [f32], output_buffer: &OutputBuffer) -> usize {
    let number_of_outputs = min(buffer.len(), output_buffer.output_neurons.len());

    for (i, buffer) in buffer.iter_mut().enumerate().take(number_of_outputs) {
        *buffer = (output_buffer.output_neurons[i].output_value >= 0.5f32) as u8 as f32;
    }

    number_of_outputs
}

fn handle_int_output(buffer: &mut [f32], output_buffer: &OutputBuffer) -> usize {
    let number_of_outputs = min(buffer.len(), output_buffer.output_neurons.len() / 64);

    for (i, buffer) in buffer.iter_mut().enumerate().take(number_of_outputs) {
        let mut val: u32 = 0;

        for offset in 0..64 {
            let neuron = &output_buffer.output_neurons[i * 64 + offset];
            val = (val << 1) | ((neuron.output_value >= 0.5) as u32);
        }

        *buffer = val as f32;
    }

    number_of_outputs
}

fn handle_float_output(buffer: &mut [f32], output_buffer: &OutputBuffer) -> usize {
    let number_of_outputs = min(buffer.len(), output_buffer.output_neurons.len() / 32);

    for (i, buffer) in buffer.iter_mut().enumerate().take(number_of_outputs) {
        let mut val: u32 = 0;

        for offset in 0..32 {
            let neuron = &output_buffer.output_neurons[i * 32 + offset];
            val = (val << 1) | ((neuron.output_value >= 0.5) as u32);
        }

        *buffer = f32::from_bits(val);
    }

    number_of_outputs
}

fn handle_plain_expected(
    output_buffer: &mut OutputBuffer,
    buffer: &[f32],
    buffer_size: u64,
) -> u64 {
    let number_of_outputs = min(buffer_size, output_buffer.output_neurons.len() as u64);

    for i in 0..number_of_outputs {
        output_buffer.output_neurons[i as usize].expected_value = buffer[i as usize];
    }

    number_of_outputs
}

fn handle_bool_expected(output_buffer: &mut OutputBuffer, buffer: &[f32], buffer_size: u64) -> u64 {
    let number_of_outputs = min(buffer_size, output_buffer.output_neurons.len() as u64);

    for i in 0..number_of_outputs {
        output_buffer.output_neurons[i as usize].expected_value =
            (buffer[i as usize] >= 0.5f32) as u8 as f32;
    }

    number_of_outputs
}

fn handle_int_expected(output_buffer: &mut OutputBuffer, buffer: &[f32], buffer_size: u64) -> u64 {
    let number_of_outputs = min(buffer_size, output_buffer.output_neurons.len() as u64 / 64);

    for i in 0..number_of_outputs {
        let val = buffer[i as usize] as u64;

        for offset in 0..64 {
            let index = (i * 64) + (63 - offset);
            output_buffer.output_neurons[index as usize].expected_value =
                ((val >> offset) & 1) as u8 as f32;
        }
    }

    number_of_outputs
}

fn handle_float_expected(
    output_buffer: &mut OutputBuffer,
    buffer: &[f32],
    buffer_size: u64,
) -> u64 {
    let number_of_outputs = min(buffer_size, output_buffer.output_neurons.len() as u64 / 32);

    for i in 0..number_of_outputs {
        let val = buffer[i as usize].to_bits();

        for offset in 0..32 {
            let index = (i * 32) + (31 - offset);
            output_buffer.output_neurons[index as usize].expected_value =
                ((val >> offset) & 1) as u8 as f32;
        }
    }

    number_of_outputs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let hexagon_uuid = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &cluster_uuid,
            &OutputType::PlainOutput,
            &finish_counter,
        );
        output_buffer.update_buffer(4);

        let mut buffer: Vec<f32> = Vec::new();
        buffer.resize(4, 0.0f32);

        {
            output_buffer.output_neurons[0].output_value = 42.0f32;
            output_buffer.output_neurons[1].output_value = 43.0f32;
            output_buffer.output_neurons[2].output_value = 44.0f32;
            output_buffer.output_neurons[3].output_value = 45.0f32;
        }

        convert_output_to_buffer(&mut buffer, &mut output_buffer);

        assert_eq!(buffer.len(), 4);
        assert_eq!(buffer[0], 42.0f32);
        assert_eq!(buffer[1], 43.0f32);
        assert_eq!(buffer[2], 44.0f32);
        assert_eq!(buffer[3], 45.0f32);

        convert_buffer_to_expected(&mut output_buffer, &buffer[..], buffer.len() as u64);

        assert_eq!(buffer.len(), 4);

        {
            assert_eq!(output_buffer.output_neurons[0].expected_value, 42.0f32);
            assert_eq!(output_buffer.output_neurons[1].expected_value, 43.0f32);
            assert_eq!(output_buffer.output_neurons[2].expected_value, 44.0f32);
            assert_eq!(output_buffer.output_neurons[3].expected_value, 45.0f32);
        }
    }

    #[test]
    fn test_bool() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let hexagon_uuid = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &cluster_uuid,
            &OutputType::BoolOutput,
            &finish_counter,
        );
        output_buffer.update_buffer(4);

        let mut buffer: Vec<f32> = Vec::new();
        buffer.resize(4, 0.0f32);

        {
            output_buffer.output_neurons[0].output_value = 0.1f32;
            output_buffer.output_neurons[1].output_value = 0.6f32;
            output_buffer.output_neurons[2].output_value = 0.3f32;
            output_buffer.output_neurons[3].output_value = 0.8f32;
        }

        convert_output_to_buffer(&mut buffer, &mut output_buffer);

        assert_eq!(buffer.len(), 4);
        assert_eq!(buffer[0], 0.0f32);
        assert_eq!(buffer[1], 1.0f32);
        assert_eq!(buffer[2], 0.0f32);
        assert_eq!(buffer[3], 1.0f32);

        convert_buffer_to_expected(&mut output_buffer, &buffer[..], buffer.len() as u64);

        assert_eq!(buffer.len(), 4);

        {
            assert_eq!(output_buffer.output_neurons[0].expected_value, 0.0f32);
            assert_eq!(output_buffer.output_neurons[1].expected_value, 1.0f32);
            assert_eq!(output_buffer.output_neurons[2].expected_value, 0.0f32);
            assert_eq!(output_buffer.output_neurons[3].expected_value, 1.0f32);
        }
    }

    #[test]
    fn test_float() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let hexagon_uuid = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &cluster_uuid,
            &OutputType::FloatOutput,
            &finish_counter,
        );
        output_buffer.update_buffer(2);

        let mut buffer: Vec<f32> = Vec::new();
        buffer.resize(2, 0.0f32);

        {
            assert_eq!(output_buffer.output_neurons.len(), 64);
            output_buffer.output_neurons[15].output_value = 0.6f32;
            output_buffer.output_neurons[16].output_value = 0.1f32;
            output_buffer.output_neurons[42].output_value = 0.3f32;
            output_buffer.output_neurons[43].output_value = 0.8f32;
        }

        convert_output_to_buffer(&mut buffer, &mut output_buffer);

        assert_eq!(buffer.len(), 2);

        convert_buffer_to_expected(&mut output_buffer, &buffer[..], buffer.len() as u64);

        assert_eq!(buffer.len(), 2);

        {
            assert_eq!(output_buffer.output_neurons[15].expected_value, 1.0f32);
            assert_eq!(output_buffer.output_neurons[16].expected_value, 0.0f32);
            assert_eq!(output_buffer.output_neurons[42].expected_value, 0.0f32);
            assert_eq!(output_buffer.output_neurons[43].expected_value, 1.0f32);
        }
    }

    #[test]
    fn test_int() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let hexagon_uuid = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &cluster_uuid,
            &OutputType::IntOutput,
            &finish_counter,
        );
        output_buffer.update_buffer(2);

        let mut buffer: Vec<f32> = Vec::new();
        buffer.resize(2, 0.0f32);

        {
            assert_eq!(output_buffer.output_neurons.len(), 128);
            output_buffer.output_neurons[62].output_value = 0.6f32;
            output_buffer.output_neurons[63].output_value = 0.1f32;
            output_buffer.output_neurons[126].output_value = 0.3f32;
            output_buffer.output_neurons[127].output_value = 0.8f32;
        }

        convert_output_to_buffer(&mut buffer, &mut output_buffer);

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer[0], 2.0f32);
        assert_eq!(buffer[1], 1.0f32);

        convert_buffer_to_expected(&mut output_buffer, &buffer[..], buffer.len() as u64);

        assert_eq!(buffer.len(), 2);
        {
            assert_eq!(output_buffer.output_neurons[62].expected_value, 1.0f32);
            assert_eq!(output_buffer.output_neurons[63].expected_value, 0.0f32);
            assert_eq!(output_buffer.output_neurons[126].expected_value, 0.0f32);
            assert_eq!(output_buffer.output_neurons[127].expected_value, 1.0f32);
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let finish_counter = Arc::new(Mutex::new(FinishCounter::default()));
        let original = OutputBuffer::new(
            "test",
            &Uuid::new_v4(),
            &Uuid::new_v4(),
            &OutputType::PlainOutput,
            &finish_counter,
        );

        let cfg = bincode::config::standard();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: OutputBuffer = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;
        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
