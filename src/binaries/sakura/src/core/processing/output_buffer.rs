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
use std::cmp::min;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use ainari_common::enums::*;

use crate::core::processing::finish_counter::FinishCounter;
use crate::core::processing::worker_queue::*;

use super::super::blocks::block_trait::*;
use super::super::blocks::output_block::*;

/// A buffer structure for storing output data from neural network processing.
/// This structure holds the output neurons, their types, and various metadata
/// needed for processing and backpropagation.
#[derive(Debug, Serialize, Deserialize)]
pub struct OutputBuffer {
    /// Unique identifier for this output buffer
    #[allow(dead_code)]
    pub uuid: Uuid,
    /// UUID of the hexagon this buffer belongs to
    #[allow(dead_code)]
    pub hexagon_uuid: Uuid,
    /// UUID of the model this buffer belongs to
    pub model_uuid: Uuid,
    /// Name of this output buffer
    pub name: String,

    /// Collection of output neurons containing the processed data
    pub output_neurons: Vec<OutputNeuron>,
    /// Type of output data this buffer holds
    pub output_type: OutputType,
    /// Size of the output data in bytes
    pub output_size: u64,

    /// Flag indicating whether this buffer has been finalized
    pub already_finalized: bool,
    /// Number of blocks connected to this output buffer
    pub number_of_connected_blocks: u64,
    /// Local counter for tracking completion status
    pub local_finish_counter: u64,
    /// Shared counter for tracking overall processing completion
    #[serde(skip, default = "init_finish_counter")]
    pub finish_counter_mutex: Arc<Mutex<FinishCounter>>,
    /// List of blocks that haven't completed processing
    #[serde(skip, default = "init_unfinished_blocks")]
    pub unfinished_blocks: Vec<Arc<Mutex<dyn Block>>>,
}

impl PartialEq for OutputBuffer {
    /// Compares two OutputBuffers for equality based on their fields.
    /// This is used to determine if two buffers represent the same logical output.
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
            && self.hexagon_uuid == other.hexagon_uuid
            && self.model_uuid == other.model_uuid
            && self.name == other.name
            && self.output_neurons == other.output_neurons
            && self.output_type == other.output_type
            && self.output_size == other.output_size
            && self.already_finalized == other.already_finalized
            && self.number_of_connected_blocks == other.number_of_connected_blocks
            && self.local_finish_counter == other.local_finish_counter
    }
}

/// Initializes a new FinishCounter wrapped in an Arc<Mutex>
/// This provides thread-safe access to the counter for tracking processing completion.
fn init_finish_counter() -> Arc<Mutex<FinishCounter>> {
    Arc::new(Mutex::new(FinishCounter::default()))
}

/// Initializes an empty vector for tracking unfinished blocks
/// This vector will hold blocks that haven't completed processing yet.
fn init_unfinished_blocks() -> Vec<Arc<Mutex<dyn Block>>> {
    Vec::new()
}

impl OutputBuffer {
    /// Creates a new OutputBuffer with the given parameters.
    /// This initializes all fields to their default values and sets up the basic structure.
    pub fn new(
        name: &str,
        hexagon_uuid: &Uuid,
        model_uuid: &Uuid,
        output_type: &OutputType,
        finish_counter_mutex: &Arc<Mutex<FinishCounter>>,
    ) -> Self {
        OutputBuffer {
            uuid: *hexagon_uuid,
            hexagon_uuid: *hexagon_uuid,
            model_uuid: *model_uuid,
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

    /// Updates the buffer size and allocates space for the specified number of outputs.
    /// This resizes the output_neurons vector to accommodate the new size and adjusts
    /// the size based on the output type (float or int outputs require more space).
    pub fn update_buffer(&mut self, number_of_outputs: usize) {
        let mut number_of_outputs_copy = number_of_outputs;

        if self.output_size < number_of_outputs_copy as u64 {
            self.output_size = number_of_outputs_copy as u64;

            // For float outputs, each output is represented by 32 bits (1 neuron per bit)
            if self.output_type == OutputType::FloatOutput {
                number_of_outputs_copy *= 32;
            }
            // For int outputs, each output is represented by 64 bits (1 neuron per bit)
            if self.output_type == OutputType::IntOutput {
                number_of_outputs_copy *= 64;
            }

            // Resize the output neurons vector, initializing new elements with default values
            self.output_neurons
                .resize_with(number_of_outputs_copy, OutputNeuron::default);
        }
    }

    /// Finalizes the training process by applying the sigmoid activation function
    /// to all output neurons. This transforms the raw output values into probabilities.
    pub fn finalize_train(&mut self) {
        for out in self.output_neurons.iter_mut() {
            if out.output_value != 0.0f32 {
                // Apply sigmoid function: 1 / (1 + e^(-x))
                out.output_value = 1.0f32 / (1.0f32 + (-out.output_value).exp());
            }
        }

        self.already_finalized = true;
    }

    /// Finalizes the processing by applying the sigmoid activation function
    /// and clearing the list of unfinished blocks.
    pub fn finalize_processing(&mut self) {
        for out in self.output_neurons.iter_mut() {
            if out.output_value != 0.0f32 {
                // Apply sigmoid function: 1 / (1 + e^(-x))
                out.output_value = 1.0f32 / (1.0f32 + (-out.output_value).exp());
            }
        }

        self.already_finalized = true;
        self.unfinished_blocks.clear();
    }

    /// Performs backpropagation by calculating the error for each output neuron
    /// and scheduling backpropagation tasks for connected blocks.
    pub fn backpropagate(&mut self, cycle_number: u64) {
        // Calculate the error for each output neuron
        for out in self.output_neurons.iter_mut() {
            let delta = out.output_value - out.expected_value;
            // Calculate the gradient for backpropagation
            out.expected_value = delta * out.output_value * (1.0f32 - out.output_value);
        }

        // Get the worker queue to schedule backpropagation tasks
        let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
        for block in self.unfinished_blocks.iter() {
            let worker_task = WorkerTask {
                task_type: WorkerTaskType::Backpropagate,
                block: Arc::clone(block),
                cycle_number,
            };

            // Add the task to the worker queue
            worker_queue.add(worker_task);
        }
        self.unfinished_blocks.clear();
    }

    /// Resets the output values of all neurons to 0 and resets the local finish counter.
    pub fn reset_output(&mut self) {
        for out in self.output_neurons.iter_mut() {
            out.output_value = 0.0f32;
        }
        self.local_finish_counter = 0;
    }

    /// Serializes the OutputBuffer to a byte vector using bincode.
    /// This allows the buffer to be stored or transmitted efficiently.
    pub fn serailize(&self) -> Vec<u8> {
        let cfg = bincode::config::standard();
        bincode::serde::encode_to_vec(self, cfg).expect("Failed to serialize")
    }

    /// Updates the finish counter and checks if all connected blocks have completed processing.
    /// Returns true if the buffer is ready for the next processing cycle.
    pub fn update_finish_counter(&mut self, cycle_number: u64) -> bool {
        let finish_counter = self.finish_counter_mutex.lock().expect("mutex poisoned");
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

/// Converts the output buffer's data to a flat buffer of f32 values.
/// The conversion depends on the output type (plain, bool, int, or float).
/// Returns the number of elements written to the buffer.
pub fn convert_output_to_buffer(buffer: &mut Vec<f32>, output_buffer: &mut OutputBuffer) -> usize {
    output_buffer.already_finalized = false;
    match output_buffer.output_type {
        OutputType::PlainOutput => handle_plain_output(buffer, output_buffer),
        OutputType::BoolOutput => handle_bool_output(buffer, output_buffer),
        OutputType::IntOutput => handle_int_output(buffer, output_buffer),
        OutputType::FloatOutput => handle_float_output(buffer, output_buffer),
    }
}

/// Converts a flat buffer of f32 values to expected values in the output buffer.
/// The conversion depends on the output type (plain, bool, int, or float).
/// Returns the number of elements read from the buffer.
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

/// Handles conversion of plain output type to a flat buffer.
/// Copies the output values directly to the buffer.
fn handle_plain_output(buffer: &mut Vec<f32>, output_buffer: &OutputBuffer) -> usize {
    buffer.resize(output_buffer.output_neurons.len(), 0.0f32);

    let number_of_outputs = min(buffer.len(), output_buffer.output_neurons.len());

    for (i, buffer) in buffer.iter_mut().enumerate().take(number_of_outputs) {
        *buffer = output_buffer.output_neurons[i].output_value;
    }

    number_of_outputs
}

/// Handles conversion of bool output type to a flat buffer.
/// Converts output values to 0.0 or 1.0 based on a threshold of 0.5.
fn handle_bool_output(buffer: &mut Vec<f32>, output_buffer: &OutputBuffer) -> usize {
    buffer.resize(output_buffer.output_neurons.len(), 0.0f32);

    let number_of_outputs = min(buffer.len(), output_buffer.output_neurons.len());

    for (i, buffer) in buffer.iter_mut().enumerate().take(number_of_outputs) {
        *buffer = (output_buffer.output_neurons[i].output_value >= 0.5f32) as u8 as f32;
    }

    number_of_outputs
}

/// Handles conversion of int output type to a flat buffer.
/// Combines 64 neurons into a single integer value.
fn handle_int_output(buffer: &mut Vec<f32>, output_buffer: &OutputBuffer) -> usize {
    buffer.resize(output_buffer.output_neurons.len() / 64, 0.0f32);
    let number_of_outputs = min(buffer.len(), output_buffer.output_neurons.len() / 64);

    for (i, buffer) in buffer.iter_mut().enumerate().take(number_of_outputs) {
        let mut val: u64 = 0;

        for offset in 0..64 {
            let neuron = &output_buffer.output_neurons[i * 64 + offset];
            val = (val << 1) | ((neuron.output_value >= 0.50) as u64);
        }

        *buffer = val as f32;
    }

    number_of_outputs
}

/// Handles conversion of float output type to a flat buffer.
/// Combines 32 neurons into a single float value using bit packing.
fn handle_float_output(buffer: &mut Vec<f32>, output_buffer: &OutputBuffer) -> usize {
    buffer.resize(output_buffer.output_neurons.len() / 32, 0.0f32);
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

/// Handles setting expected values for plain output type.
/// Copies the values directly from the buffer to the expected values.
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

/// Handles setting expected values for bool output type.
/// Converts values to 0.0 or 1.0 based on a threshold of 0.5.
fn handle_bool_expected(output_buffer: &mut OutputBuffer, buffer: &[f32], buffer_size: u64) -> u64 {
    let number_of_outputs = min(buffer_size, output_buffer.output_neurons.len() as u64);

    for i in 0..number_of_outputs {
        output_buffer.output_neurons[i as usize].expected_value =
            (buffer[i as usize] >= 0.5f32) as u8 as f32;
    }

    number_of_outputs
}

/// Handles setting expected values for int output type.
/// Expands a single integer value into 64 neurons.
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

/// Handles setting expected values for float output type.
/// Expands a single float value into 32 neurons using bit unpacking.
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
        let model_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &model_uuid,
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
        let model_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &model_uuid,
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
        let model_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &model_uuid,
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
        let model_uuid = Uuid::new_v4();
        let mut output_buffer = OutputBuffer::new(
            "test",
            &hexagon_uuid,
            &model_uuid,
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
