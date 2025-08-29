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

use bytemuck::cast_slice;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

use ainari_api::structs::task_structs::TaskState;
use ainari_common::error::AinariError;
use ainari_dataset::dataset_io::{DataSetFileReadHandleV1_0, DataSetFileWriteHandleV1_0};

use crate::api::user_context::UserContext;
use crate::core::blocks::block_trait::*;
use crate::core::cluster_handler::*;
use crate::database::checkpoint_table;
use crate::database::task_table;

use super::super::processing::output_buffer::*;
use super::super::processing::worker_queue::*;

#[derive(Debug)]
pub struct TrainInfo {
    pub inputs: HashMap<String, DataSetFileReadHandleV1_0>,
    pub outputs: HashMap<String, DataSetFileReadHandleV1_0>,
}

#[derive(Debug)]
pub struct RequestInfo {
    pub inputs: HashMap<String, DataSetFileReadHandleV1_0>,
    pub results: DataSetFileWriteHandleV1_0,
}

#[derive(Debug)]
pub struct CheckpointSaveInfo {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct CheckpointRestoreInfo {
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum TaskVariant {
    Training(TrainInfo),
    Request(RequestInfo),
    CheckpointSave(CheckpointSaveInfo),
    CheckpointRestore(CheckpointRestoreInfo),
}

#[derive(Debug)]
pub struct TaskMeta {
    pub number_of_cycles: u64,
    pub number_of_epochs: u64,
    pub number_of_finished_cycles: u64,
    pub number_of_finished_epochs: u64,
    pub time_length: u64,

    pub task_cycle_counter: u64,

    pub is_finished: bool,
    pub prev_timestamp: std::time::Instant,
}

impl TaskMeta {
    pub fn new(number_of_cycler_per_epoch: u64, number_of_epochs: u64, time_length: u64) -> Self {
        Self {
            number_of_cycles: number_of_cycler_per_epoch,
            number_of_epochs,
            number_of_finished_cycles: 0,
            number_of_finished_epochs: 0,
            time_length,

            task_cycle_counter: 0,

            is_finished: false,
            prev_timestamp: std::time::Instant::now(),
        }
    }
}

#[derive(Debug)]
pub struct Task {
    pub uuid: Uuid,
    pub cluster_uuid: Uuid,
    pub name: String,
    pub user_id: String,
    pub project_id: String,

    pub info: TaskVariant,
    pub meta: TaskMeta,
}

impl Task {
    // ==================================================================================================

    pub fn start_task(&mut self) -> bool {
        // check if task was aborted
        if task_table::is_aborted(&self.uuid) {
            return true;
        }

        {
            let cluster_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");
            let _ = cluster_handler.reset_outputs(&self.cluster_uuid);
        }

        self.meta.prev_timestamp = Instant::now();
        let _ = task_table::update_task_state(&self.uuid, &TaskState::Active);

        match &mut self.info {
            TaskVariant::Training(task_info) => {
                run_train_task_cycle(&self.uuid, &self.cluster_uuid, &mut self.meta, task_info);
                true
            }
            TaskVariant::Request(task_info) => {
                run_request_task_cycle(&self.uuid, &self.cluster_uuid, &mut self.meta, task_info);
                true
            }
            TaskVariant::CheckpointSave(task_info) => {
                handle_checkpoint_save_task(
                    &self.uuid,
                    &self.cluster_uuid,
                    &self.name,
                    &self.user_id,
                    &self.project_id,
                    &mut self.meta,
                    task_info,
                );
                false
            }
            TaskVariant::CheckpointRestore(task_info) => {
                handle_checkpoint_restore_task(
                    &self.uuid,
                    &self.cluster_uuid,
                    &mut self.meta,
                    task_info,
                );
                false
            }
        }
    }

    pub fn finalize_task(&mut self) {
        let _ = task_table::update_task_state(&self.uuid, &TaskState::Finished);
        let _ = task_table::update_task_progress(
            &self.uuid,
            &(self.meta.number_of_epochs as i64),
            &(self.meta.number_of_cycles as i64),
        );
    }

    pub fn finish_cycle(&mut self) {
        match &mut self.info {
            TaskVariant::Training(task_info) => {
                finish_train_cycle(&self.uuid, task_info);
            }
            TaskVariant::Request(task_info) => {
                finish_request_cycle(&self.uuid, &self.cluster_uuid, task_info);
            }
            _ => {
                return;
            }
        }

        // update current state in database at least after 1 second
        let now = Instant::now();
        if now.duration_since(self.meta.prev_timestamp) >= Duration::from_secs(1) {
            self.meta.prev_timestamp = now;
            let _ = task_table::update_task_progress(
                &self.uuid,
                &(self.meta.number_of_finished_epochs as i64),
                &(self.meta.number_of_finished_cycles as i64),
            );
            if task_table::is_aborted(&self.uuid) {
                // TODO: handle abort correctly
                return;
            }
        }

        // update and check cycle- and epoch-counter
        self.meta.number_of_finished_cycles += 1;
        if self.meta.number_of_finished_cycles == self.meta.number_of_cycles {
            self.meta.number_of_finished_epochs += 1;
            if self.meta.number_of_finished_epochs == self.meta.number_of_epochs {
                self.meta.is_finished = true;
                return;
            } else {
                self.meta.number_of_finished_cycles = 0;
            }
        }
        self.meta.task_cycle_counter += 1;

        // run next-cycle
        match &mut self.info {
            TaskVariant::Training(task_info) => {
                run_train_task_cycle(&self.uuid, &self.cluster_uuid, &mut self.meta, task_info);
            }
            TaskVariant::Request(task_info) => {
                run_request_task_cycle(&self.uuid, &self.cluster_uuid, &mut self.meta, task_info);
            }
            _ => {}
        }
    }

    pub fn is_task_finished(&self) -> bool {
        self.meta.is_finished
    }
}

fn finish_train_cycle(_: &Uuid, _: &mut TrainInfo) {}

fn finish_request_cycle(task_uuid: &Uuid, cluster_uuid: &Uuid, task_info: &mut RequestInfo) {
    // get output-values form backend and write them into the dataset
    match write_output_into_dataset(cluster_uuid, &mut task_info.results) {
        Ok(()) => {}
        Err(AinariError::InvalidInput(msg)) => {
            let _ = task_table::set_error_state(task_uuid, &msg);
        }
        Err(AinariError::Error(msg)) => {
            log::error!("Error while writing output into dataset: {msg}");
            let db_msg = "internal error".to_string();
            let _ = task_table::set_error_state(task_uuid, &db_msg);
        }
    }
}

fn run_train_task_cycle(
    task_uuid: &Uuid,
    cluster_uuid: &Uuid,
    meta: &mut TaskMeta,
    task_info: &mut TrainInfo,
) {
    // update current state in database at least after 1 second
    let now = Instant::now();
    if now.duration_since(meta.prev_timestamp) >= Duration::from_secs(1) {
        meta.prev_timestamp = now;
        let _ = task_table::update_task_progress(
            task_uuid,
            &(meta.number_of_finished_epochs as i64),
            &(meta.number_of_finished_cycles as i64),
        );

        // check if task was aborted
        if task_table::is_aborted(task_uuid) {
            return;
        }
    }

    // push output-values form dataset into the backend
    for (hexagon_name, file_handle) in &mut task_info.outputs {
        match apply_dataset_to_expected(
            cluster_uuid,
            hexagon_name,
            file_handle,
            meta.number_of_finished_cycles,
            meta.time_length,
        ) {
            Ok(()) => {}
            Err(AinariError::InvalidInput(msg)) => {
                let _ = task_table::set_error_state(task_uuid, &msg);
                return;
            }
            Err(AinariError::Error(msg)) => {
                log::error!("{msg}");
                let db_msg = "internal error".to_string();
                let _ = task_table::set_error_state(task_uuid, &db_msg);
                return;
            }
        }
    }

    // push input-values form dataset into the backend
    for (hexagon_name, file_handle) in &mut task_info.inputs {
        match apply_dataset_to_input(
            cluster_uuid,
            hexagon_name,
            file_handle,
            meta.number_of_finished_cycles,
            meta.time_length,
            &WorkerTaskType::Train,
            meta.task_cycle_counter,
        ) {
            Ok(()) => {}
            Err(AinariError::InvalidInput(msg)) => {
                let _ = task_table::set_error_state(task_uuid, &msg);
                return;
            }
            Err(AinariError::Error(msg)) => {
                log::error!("{msg}");
                let db_msg = "internal error".to_string();
                let _ = task_table::set_error_state(task_uuid, &db_msg);
                return;
            }
        }
    }
}

fn run_request_task_cycle(
    task_uuid: &Uuid,
    cluster_uuid: &Uuid,
    meta: &mut TaskMeta,
    task_info: &mut RequestInfo,
) {
    // update current state in database at least after 1 second
    let now = Instant::now();
    if now.duration_since(meta.prev_timestamp) >= Duration::from_secs(1) {
        meta.prev_timestamp = now;
        let _ = task_table::update_task_progress(
            task_uuid,
            &0,
            &(meta.number_of_finished_cycles as i64),
        );

        // check if task was aborted
        if task_table::is_aborted(task_uuid) {
            return;
        }
    }

    // push input-values form dataset into the backend
    for (hexagon_name, file_handle) in &mut task_info.inputs {
        match apply_dataset_to_input(
            cluster_uuid,
            hexagon_name,
            file_handle,
            meta.number_of_finished_cycles,
            meta.time_length,
            &WorkerTaskType::Process,
            meta.number_of_finished_cycles,
        ) {
            Ok(()) => {}
            Err(AinariError::InvalidInput(msg)) => {
                let _ = task_table::set_error_state(task_uuid, &msg);
                return;
            }
            Err(AinariError::Error(msg)) => {
                log::error!("{msg}");
                let db_msg = "internal error".to_string();
                let _ = task_table::set_error_state(task_uuid, &db_msg);
                return;
            }
        }
    }
}

pub fn apply_plain_input(
    cluster_uuid: &Uuid,
    hexagon_name: &String,
    input_ptr: &[f32],
    input_size: u64,
    pos_counter: usize,
    time_length: u64,
    task_type: &WorkerTaskType,
) -> Result<(), AinariError> {
    let cluster_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");
    let input_block_mutex = cluster_handler.get_input_block(cluster_uuid, hexagon_name)?;

    let mut input_block = input_block_mutex.lock().expect("mutex poisoned");
    let allow_creation = *task_type == WorkerTaskType::Train;
    input_block.apply_input(
        input_ptr,
        input_size as usize,
        pos_counter,
        time_length as usize,
        allow_creation,
    );

    let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
    let cycle_number = 0;
    let worker_task = WorkerTask {
        task_type: task_type.clone(),
        block: Arc::clone(&input_block_mutex) as Arc<Mutex<dyn Block>>,
        cycle_number,
    };
    worker_queue.add(worker_task);

    Ok(())
}

fn apply_dataset_to_input(
    cluster_uuid: &Uuid,
    hexagon_name: &String,
    file_handle: &mut DataSetFileReadHandleV1_0,
    cycle_count: u64,
    time_length: u64,
    task_type: &WorkerTaskType,
    task_cycle_counter: u64,
) -> Result<(), AinariError> {
    // get input-block
    let cluster_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");
    let input_block_mutex = cluster_handler.get_input_block(cluster_uuid, hexagon_name)?;
    drop(cluster_handler);

    let mut input_block = input_block_mutex.lock().expect("mutex poisoned");

    // fill input with data from dataset
    let mut pos_counter: usize = 0;
    for time_point in 0..time_length {
        let (input_ptr, input_size) =
            file_handle.get_data_from_file(&(cycle_count + time_point))?;
        let allow_creation = *task_type == WorkerTaskType::Train;
        input_block.apply_input(
            input_ptr,
            input_size as usize,
            pos_counter,
            time_length as usize,
            allow_creation,
        );

        pos_counter += input_size as usize;
    }

    // add input-block to worker-queue
    let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
    let worker_task = WorkerTask {
        task_type: task_type.clone(),
        block: Arc::clone(&input_block_mutex) as Arc<Mutex<dyn Block>>,
        cycle_number: task_cycle_counter,
    };
    worker_queue.add(worker_task);

    Ok(())
}

pub fn apply_expected(
    cluster_uuid: &Uuid,
    hexagon_name: &String,
    input_ptr: &[f32],
    input_size: u64,
) -> Result<(), AinariError> {
    let cluster_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");
    let output_buffer_mutex = cluster_handler.get_output_buffer(cluster_uuid, hexagon_name)?;

    let mut output_buffer = output_buffer_mutex.lock().expect("mutex poisoned");
    output_buffer.reset_output();
    convert_buffer_to_expected(&mut output_buffer, input_ptr, input_size);

    Ok(())
}

fn apply_dataset_to_expected(
    cluster_uuid: &Uuid,
    hexagon_name: &String,
    file_handle: &mut DataSetFileReadHandleV1_0,
    cycle_count: u64,
    time_length: u64,
) -> Result<(), AinariError> {
    let (input_ptr, input_size) =
        file_handle.get_data_from_file(&(cycle_count + time_length - 1))?;

    apply_expected(cluster_uuid, hexagon_name, input_ptr, input_size)?;

    Ok(())
}

fn write_output_into_dataset(
    cluster_uuid: &Uuid,
    file_handle: &mut DataSetFileWriteHandleV1_0,
) -> Result<(), AinariError> {
    let cluster_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");

    // get column-description from the dataset
    for (hexagon_name, col_get) in &file_handle.header.columns {
        let size_output = (col_get.end - col_get.start) as usize;
        let mut output_read = vec![0.0f32; size_output];

        let output_buffer_mutex = cluster_handler.get_output_buffer(cluster_uuid, hexagon_name)?;

        let mut output_buffer = output_buffer_mutex.lock().expect("mutex poisoned");
        convert_output_to_buffer(&mut output_read, &mut output_buffer);
        output_buffer.reset_output();

        let output_bytes = cast_slice(&output_read);
        let _ = file_handle.target_file.write_all(output_bytes);
    }

    Ok(())
}

fn handle_checkpoint_save_task(
    task_uuid: &Uuid,
    cluster_uuid: &Uuid,
    task_name: &str,
    user_id: &str,
    project_id: &str,
    _: &mut TaskMeta,
    task_info: &mut CheckpointSaveInfo,
) {
    let cluster_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");
    match cluster_handler.create_checkpoint(cluster_uuid, &task_info.path) {
        Ok(()) => {}
        Err(_) => {
            return;
        }
    }

    // create information for new database-entry
    let context = &UserContext {
        user_id: user_id.to_owned(),
        project_id: project_id.to_owned(),
        is_admin: false,
        is_project_admin: false,
    };

    // add information of new checkpoint to the database
    // HINT (kitsudaiki): It is intended that the task-uuid is also the checkpoint-uuid, because of easier identification
    let file_path_str: String = task_info.path.to_string_lossy().into();
    match checkpoint_table::add_new_checkpoint(task_uuid, task_name, &file_path_str, context) {
        Ok(_) => {}
        Err(e) => {
            log::error!("{e}");
            let _ = task_table::set_error_state(task_uuid, &"Internal error".to_string());
            return;
        }
    }

    let _ = task_table::update_task_state(task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &1, &1);
}

fn handle_checkpoint_restore_task(
    task_uuid: &Uuid,
    cluster_uuid: &Uuid,
    _: &mut TaskMeta,
    task_info: &mut CheckpointRestoreInfo,
) {
    let mut cluster_handler = CLUSTER_HANDLER.write().expect("mutex poisoned");
    match cluster_handler.restore_checkpoint(cluster_uuid, &task_info.path) {
        Ok(()) => {}
        Err(_) => {
            return;
        }
    }

    let _ = task_table::update_task_state(task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &1, &1);
}
