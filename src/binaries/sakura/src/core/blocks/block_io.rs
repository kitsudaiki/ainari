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
use std::sync::Arc;
use uuid::Uuid;

use crate::core::blocks::target_search::*;
use crate::core::model_handler::*;
use crate::core::processing::worker_queue::*;

use ainari_common::constants::*;
use ainari_common::error::AinariError;

use super::axons::*;

/// A buffer structure for managing input and output axon sections in a neural block.
///
/// This structure maintains buffers for both input and output axon sections, along with
/// counters to track their usage and state.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockIoBuffer {
    /// Buffer for incoming axon sections
    pub input_buffer: Vec<AxonSection>,
    /// Buffer for outgoing axon sections
    pub output_buffer: Vec<AxonSection>,

    /// Counter for tracking the number of inputs currently in use
    pub input_buffer_counter: u64,
    /// Counter for tracking the number of outputs currently in use
    pub output_buffer_counter: u64,

    /// Total number of inputs currently being used
    pub inputs_in_use: u64,
}

/// Connects the output axon sections of a block to their target destinations.
///
/// This function iterates through all output axon sections and:
/// 1. For uninitialized sections, sets up their source information and connects them to new targets
/// 2. For initialized sections, ensures they have valid source and target block references
///
/// # Arguments
///
/// * `io_buffer` - Mutable reference to the block's IO buffer
/// * `model_uuid` - UUID of the model containing this block
/// * `source_hexagon_uuid` - UUID of the hexagon containing this block
/// * `source_block_uuid` - UUID of this block
///
/// # Returns
///
/// * `Result<(), AinariError>` - Returns Ok(()) on success or an error if connection fails
pub fn connect_outputs(
    axon_section: &mut AxonSection,
    model_uuid: &Uuid,
    source_hexagon_uuid: &Uuid,
    source_block_uuid: &Uuid,
    source_pos: u8,
) -> Result<bool, AinariError> {
    if axon_section.target_pos == UNINIT_STATE_8 {
        // let mut model_handler = MODEL_HANDLER.write().expect("mutex poisoned");

        // set source-values for the axon-section
        axon_section.model_uuid = *model_uuid;
        axon_section.source_hexagon_uuid = *source_hexagon_uuid;
        axon_section.source_block_uuid = *source_block_uuid;
        axon_section.source_pos = source_pos;

        // model_handler.get_target(axon_section);
        return connect_to_new_target(axon_section);
    } else if axon_section.source_block.is_none() || axon_section.target_block.is_none() {
        // Get model handler to fetch block references
        let model_handler = MODEL_HANDLER.read().expect("mutex poisoned");
        axon_section.model_uuid = *model_uuid;
        axon_section.source_block = Some(model_handler.get_block(
            model_uuid,
            &axon_section.source_hexagon_uuid,
            &axon_section.source_block_uuid,
        )?);
        axon_section.target_block = Some(model_handler.get_block(
            model_uuid,
            &axon_section.target_hexagon_uuid,
            &axon_section.target_block_uuid,
        )?);
    }

    Ok(true)
}

/// Sends the output axon sections to their target blocks and schedules processing tasks.
///
/// This function:
/// 1. Iterates through all output axon sections
/// 2. Sends each section to its target block
/// 3. When all required inputs are received, schedules a worker task for the target block
///
/// # Arguments
///
/// * `io_buffer` - Reference to the block's IO buffer
/// * `task_type` - Type of worker task to schedule
/// * `cycle_number` - Current cycle number for task tracking
pub fn send_forward(
    io_buffer: &mut BlockIoBuffer,
    task_type: WorkerTaskType,
    cycle_number: u64,
    model_uuid: &Uuid,
    source_hexagon_uuid: &Uuid,
    source_block_uuid: &Uuid,
) {
    // send outputs to target
    for (i, axon_section) in io_buffer.output_buffer.iter_mut().enumerate() {
        let forward_allowed = connect_outputs(
            axon_section,
            model_uuid,
            source_hexagon_uuid,
            source_block_uuid,
            i as u8,
        )
        .unwrap_or(true);

        // Get the target block mutex or skip this axon section
        let target_block_mutex = if let Some(t) = &axon_section.target_block {
            t
        } else {
            continue;
        };

        // Lock the target block and update its input buffer
        let mut target_block = target_block_mutex.lock().expect("mutex poisoned");
        let target_bock_io = target_block.get_block_io();
        let is_done = target_bock_io.input_buffer[axon_section.target_pos as usize].done;
        target_bock_io.input_buffer[axon_section.target_pos as usize] = axon_section.clone();
        target_bock_io.input_buffer[axon_section.target_pos as usize].done = is_done;
        target_bock_io.input_buffer_counter += 1;

        // Check if all required inputs are present and schedule a worker task if so
        if target_bock_io.input_buffer_counter >= target_bock_io.inputs_in_use {
            target_bock_io.input_buffer_counter = 0;

            if forward_allowed {
                let worker_task = WorkerTask {
                    task_type: task_type.clone(),
                    block: Arc::clone(target_block_mutex),
                    cycle_number,
                };

                // Add the task to the worker queue
                let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
                worker_queue.add(worker_task);
            }
        }
    }
}

/// Sends input axon sections back to their source blocks for backpropagation.
///
/// This function:
/// 1. Iterates through all input axon sections
/// 2. Sends each section back to its source block
/// 3. When all required outputs are collected, schedules a backpropagation task
///
/// # Arguments
///
/// * `io_buffer` - Mutable reference to the block's IO buffer
/// * `cycle_number` - Current cycle number for task tracking
///
/// # Returns
///
/// * `bool` - Returns true if all sections were processed successfully
pub fn send_backward(io_buffer: &mut BlockIoBuffer, cycle_number: u64) -> bool {
    for axon_section in io_buffer.input_buffer.iter_mut() {
        // Get the source block mutex or skip this axon section
        let source_block_mutex = if let Some(s) = &axon_section.source_block {
            s
        } else {
            continue;
        };

        // Lock the source block and update its output buffer
        if let Ok(mut source_block) = source_block_mutex.lock() {
            let target_bock_io = source_block.get_block_io();
            target_bock_io.output_buffer[axon_section.source_pos as usize] = axon_section.clone();
            target_bock_io.output_buffer_counter += 1;

            // Check if all required outputs are collected and schedule a backpropagation task if so
            if target_bock_io.output_buffer_counter >= target_bock_io.output_buffer.len() as u64 {
                target_bock_io.output_buffer_counter = 0;

                let worker_task = WorkerTask {
                    task_type: WorkerTaskType::Backpropagate,
                    block: Arc::clone(source_block_mutex),
                    cycle_number,
                };

                // Add the task to the worker queue
                let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
                worker_queue.add(worker_task);
            }
            axon_section.done = true;
        }
    }

    true
}

/// Sends input axon sections back to their source blocks for backpropagation with retry logic.
///
/// This function is similar to `send_backward` but includes retry logic for locked blocks.
/// It will attempt to lock each source block, and if a lock cannot be obtained, it will
/// mark the axon section as not done and return false, allowing the caller to retry later.
///
/// # Arguments
///
/// * `io_buffer` - Mutable reference to the block's IO buffer
/// * `cycle_number` - Current cycle number for task tracking
///
/// # Returns
///
/// * `bool` - Returns true if all sections were processed successfully, false if any blocks were locked
pub fn send_backward_with_retry(io_buffer: &mut BlockIoBuffer, cycle_number: u64) -> bool {
    for axon_section in io_buffer.input_buffer.iter_mut() {
        // Skip already processed axon sections
        if axon_section.done {
            continue;
        }

        // Get the source block mutex or skip this axon section
        let source_block_mutex = if let Some(s) = &axon_section.source_block {
            s
        } else {
            continue;
        };

        // Attempt to lock the source block non-blockingly
        if let Ok(mut source_block) = source_block_mutex.try_lock() {
            let target_bock_io = source_block.get_block_io();
            let is_done = target_bock_io.output_buffer[axon_section.source_pos as usize].done;
            target_bock_io.output_buffer[axon_section.source_pos as usize] = axon_section.clone();
            target_bock_io.output_buffer[axon_section.source_pos as usize].done = is_done;
            target_bock_io.output_buffer_counter += 1;

            // Check if all required outputs are collected and schedule a backpropagation task if so
            if target_bock_io.output_buffer_counter >= target_bock_io.output_buffer.len() as u64 {
                target_bock_io.output_buffer_counter = 0;

                let worker_task = WorkerTask {
                    task_type: WorkerTaskType::Backpropagate,
                    block: Arc::clone(source_block_mutex),
                    cycle_number,
                };

                // Add the task to the worker queue
                let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
                worker_queue.add(worker_task);
            }
            axon_section.done = true;
        } else {
            // Mark as not done if we couldn't get the lock
            axon_section.done = false;
            return false;
        }
    }

    // Reset the done flags for any remaining axon sections
    for axon_section in io_buffer.input_buffer.iter_mut() {
        axon_section.done = false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let mut original = BlockIoBuffer::default();
        original.input_buffer.push(AxonSection::default());

        let cfg = bincode::config::standard();
        let serialized: Vec<u8> =
            bincode::serde::encode_to_vec(&original, cfg).expect("Failed to serialize");
        let deserialized: BlockIoBuffer = bincode::serde::decode_from_slice(&serialized, cfg)
            .expect("Failed to deserialize")
            .0;

        println!("size: {}", serialized.len());

        assert_eq!(original, deserialized);
    }
}
