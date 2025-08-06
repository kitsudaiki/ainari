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
use std::sync::{Arc};
 
use crate::core::cluster_handler::*;
use crate::core::processing::worker_queue::*;

use hanami_common::constants::*;

use super::axons::*;

#[derive(Default, Clone)]
pub struct BlockIoBuffer {
    pub input_buffer: Vec<AxonSection>,
    pub output_buffer: Vec<AxonSection>,

    pub input_buffer_counter: u64,
    pub output_buffer_counter: u64,

    pub inputs_in_use: u64,
}

pub fn connect_outputs(io_buffer: &mut BlockIoBuffer, source_cluster_uuid: &Uuid, source_hexagon_uuid: &Uuid, source_block_uuid: &Uuid) {
    // in case of training, get targets for all not-connected axon-sections
    let mut cluster_handler = CLUSTER_HANDLER.write().unwrap();

    let mut counter = 0;
    for axon_section in io_buffer.output_buffer.iter_mut() {
        if axon_section.target_pos == UNINIT_STATE_8 {
            // set source-values for the axon-section
            axon_section.cluster_uuid = source_cluster_uuid.clone();
            axon_section.source_hexagon_uuid = source_hexagon_uuid.clone();
            axon_section.source_block_uuid = source_block_uuid.clone();
            axon_section.source_pos = counter;

            cluster_handler.get_target(axon_section);
        }

        counter += 1;
    }
}

pub fn send_forward(io_buffer: &BlockIoBuffer, task_type: WorkerTaskType) {
    // send outputs to target
    let mut worker_queue = WORKER_QUEUE.lock().unwrap();
    for axon_section in io_buffer.output_buffer.iter() {
        let target_block_mutex = if let Some(t) = &axon_section.target_block {
            t
        } else {
            continue;
        };
        let block_clone = Arc::clone(&target_block_mutex);
        let mut target_block = target_block_mutex.lock().unwrap();
        let target_bock_io = target_block.get_block_io();
        target_bock_io.input_buffer[axon_section.target_pos as usize] = axon_section.clone();
        target_bock_io.input_buffer_counter += 1;

        if target_bock_io.input_buffer_counter >= target_bock_io.inputs_in_use as u64 {
            target_bock_io.input_buffer_counter = 0;
            
            let worker_task = WorkerTask{
                task_type: task_type.clone(),
                block: block_clone,
            };
            
            worker_queue.add(worker_task);
        }
    }
}

pub fn send_backward(io_buffer: &BlockIoBuffer) {
    let mut worker_queue = WORKER_QUEUE.lock().unwrap();
    for axon_section in io_buffer.input_buffer.iter() {
        let source_block_mutex = if let Some(s) = &axon_section.source_block {
            s
        } else {
            continue;
        };

        // send axon-sections to target-block and create new worker-task
        let mut source_block = source_block_mutex.lock().unwrap();
        let target_bock_io = source_block.get_block_io();
        target_bock_io.output_buffer[axon_section.source_pos as usize] = axon_section.clone();
        target_bock_io.output_buffer_counter += 1;

        if target_bock_io.output_buffer_counter >= target_bock_io.output_buffer.len() as u64 {
            target_bock_io.output_buffer_counter = 0;

            let worker_task = WorkerTask{
                task_type: WorkerTaskType::Backpropagate,
                block: Arc::clone(&source_block_mutex),
            };
            
            worker_queue.add(worker_task);
        }
    }
}
