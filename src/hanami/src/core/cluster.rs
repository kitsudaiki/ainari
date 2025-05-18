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

use log::{debug, error};
use uuid::Uuid;
use bytemuck::{cast_slice, cast_slice_mut};
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::SeekFrom;
use std::io::{Read, Write, Seek};
use std::time::{Duration, Instant};
use std::collections::HashMap;

use hanami_dataset::dataset_io::{DataSetFileReadHandleV1_0, DataSetFileWriteHandleV1_0};

use crate::database::task_table;
use crate::database::checkpoint_table;
use crate::api::http_endpoints::cluster::task::task_structs::TaskState;
use crate::api::user_context::UserContext;

use super::task_queue::{TaskQueue, init_task_queue};
use super::tasks::{Task, TaskVariant, TrainInfo, RequestInfo, CheckpointSaveInfo, CheckpointRestoreInfo};

// HINT (kitsudaiki): ffi is necessary ot get the c++ stuff, defined in the lib.rs
use crate::ffi;
use autocxx::prelude::*;

pub struct ClusterLinkHanle {
    pub cluster_link: UniquePtr<ffi::ClusterLink>, 
}

fn get_values(
    hexagon_name: &String, 
    file_handle: &mut DataSetFileReadHandleV1_0, 
    cycle_count: &u64, 
    cluster_link: &mut UniquePtr<ffi::ClusterLink>, 
    is_expected: bool) -> Result<(), String> 
{
    // get column-description from the dataset
    let column = &file_handle.selected_column;
    let col_get = match file_handle.header.columns.get(column) {
        Some(col) => col,
        _ => {
            let msg = format!("Column with name '{column}' not found in dataset.");
            return Err(msg);
        }
    };

    // calculate position in dataset-file
    let size_input = (col_get.end - col_get.start) as usize;
    let mut offset_bytes = (file_handle.header.row_size) * 4 * cycle_count;
    offset_bytes += col_get.start * 4;

    let mut input_read = vec![0.0f32; size_input];
    let byte_slice_input: &mut [u8] = cast_slice_mut(input_read.as_mut_slice());
    file_handle.target_file.seek(SeekFrom::Start(file_handle.payload_offset + offset_bytes)).unwrap();
    let _ = file_handle.target_file.read_exact(byte_slice_input);
    let input_ptr: *mut f32 = input_read.as_mut_ptr();

    // tigger action in c++ code
    cxx::let_cxx_string!(cxx_name = hexagon_name); 
    if is_expected == false {
        unsafe {
            if cluster_link.pin_mut().fillInput(&cxx_name, input_ptr, size_input as u64) == false {
                let msg = format!("Hexagon with name '{hexagon_name}' not found in cluster.");
                return Err(msg);
            }
        }
    } else {
        unsafe {
            if cluster_link.pin_mut().fillExpected(&cxx_name, input_ptr, size_input as u64) == false {
                let msg = format!("Hexagon with name '{hexagon_name}' not found in cluster.");
                return Err(msg);
            }
        }
    }

    Ok(())
}

fn write_values(
    hexagon_name: &String, 
    file_handle: &mut DataSetFileWriteHandleV1_0, 
    cluster_link: &mut UniquePtr<ffi::ClusterLink>) -> Result<(), String> 
{
    // get column-description from the dataset
    let column = &file_handle.selected_column;
    let col_get = match file_handle.header.columns.get(column) {
        Some(col) => col,
        _ => {
            let msg = format!("Column with name '{column}' not found in dataset.");
            return Err(msg);
        }
    };

    let size_output = (col_get.end - col_get.start) as usize;
    let mut output_read = vec![0.0f32; size_output];
    let output_ptr: *mut f32 = output_read.as_mut_ptr();

    // get output from the c++ backend
    cxx::let_cxx_string!(cxx_name = hexagon_name); 
    unsafe {
        if cluster_link.pin_mut().getOutput(&cxx_name, output_ptr, size_output as u64) == false {
            let msg = format!("Hexagon with name '{hexagon_name}' not found in cluster.");
            return Err(msg);
        }
    }

    let output_bytes = cast_slice(&output_read);
    let _ = file_handle.target_file.write_all(&output_bytes);

    Ok(())
}

fn handle_train_task(task_uuid: &Uuid, task_info: &mut TrainInfo, link_handle: &Arc<Mutex<ClusterLinkHanle>>) {
    // check if task was aborted
    if task_table::is_aborted(&task_uuid) {
        return;
    }

    let mut prev_timestamp = Instant::now();
    let _ = task_table::update_task_state(&task_uuid, &TaskState::Active);

    for epoch_count in 0..1 {
        for cycle_count in 0..task_info.number_of_cycles {
            // update current state in database at least after 1 second
            let now = Instant::now();
            if now.duration_since(prev_timestamp) >= Duration::from_secs(1) {
                prev_timestamp = now;
                let _ = task_table::update_task_progress(&task_uuid, &(epoch_count as i64), &(cycle_count as i64));

                // check if task was aborted
                if task_table::is_aborted(&task_uuid) {
                    return;
                }
            }

            let mut link = link_handle.lock().unwrap();

            // push input-values form dataset into the backend
            for (hexagon_name, file_handle) in &mut task_info.inputs {  
                match get_values(hexagon_name, file_handle, &cycle_count, &mut link.cluster_link, false) {
                    Ok(()) => {},
                    Err(e) => {
                        let _ = task_table::set_error_state(&task_uuid, &e);
                        return;
                    }
                }
            }

            // push output-values form dataset into the backend
            for (hexagon_name, file_handle) in &mut task_info.outputs {  
                match get_values(hexagon_name, file_handle, &cycle_count, &mut link.cluster_link, true) {
                    Ok(()) => {},
                    Err(e) => {
                        let _ = task_table::set_error_state(&task_uuid, &e);
                        return;
                    }
                }
            }

            link.cluster_link.pin_mut().doTrain();

            drop(link);
        }
    }

    let _ = task_table::update_task_state(&task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &(1 as i64), &(task_info.number_of_cycles as i64));
}

fn handle_request_task(task_uuid: &Uuid, task_info: &mut RequestInfo, link_handle: &Arc<Mutex<ClusterLinkHanle>>) {
    // check if task was aborted
    if task_table::is_aborted(&task_uuid) {
        return;
    }
    
    let mut prev_timestamp = Instant::now();
    let _ = task_table::update_task_state(&task_uuid, &TaskState::Active);

    for epoch_count in 0..1 {
        for cycle_count in 0..task_info.number_of_cycles {
            // update current state in database at least after 1 second
            let now = Instant::now();
            if now.duration_since(prev_timestamp) >= Duration::from_secs(1) {
                prev_timestamp = now;
                let _ = task_table::update_task_progress(task_uuid, &(epoch_count as i64), &(cycle_count as i64));

                // check if task was aborted
                if task_table::is_aborted(&task_uuid) {
                    return;
                }
            }

            let mut link = link_handle.lock().unwrap();

            // push input-values form dataset into the backend
            for (hexagon_name, file_handle) in &mut task_info.inputs {  
                match get_values(hexagon_name, file_handle, &cycle_count, &mut link.cluster_link, false) {
                    Ok(()) => {},
                    Err(e) => {
                        let _ = task_table::set_error_state(&task_uuid, &e);
                        return;
                    }
                }
            }

            link.cluster_link.pin_mut().doRequest();

            // get output-values form backend and write them into the dataset
            for (hexagon_name, file_handle) in &mut task_info.results {  
                match write_values(hexagon_name, file_handle, &mut link.cluster_link) {
                    Ok(()) => {},
                    Err(e) => {
                        let _ = task_table::set_error_state(&task_uuid, &e);
                        return;
                    }
                }
            }

            drop(link);
        }
    }

    let _ = task_table::update_task_state(&task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &(1 as i64), &(task_info.number_of_cycles as i64));
}

fn handle_checkpoint_save_task(task_uuid: &Uuid, task_name: &String, user_id: &String, project_id: &String, task_info: &mut CheckpointSaveInfo, link_handle: &Arc<Mutex<ClusterLinkHanle>>) {
    let mut link = link_handle.lock().unwrap();

    let file_path_str: String = task_info.path.to_string_lossy().into();
    cxx::let_cxx_string!(cxx_path = file_path_str.clone());
    link.cluster_link.pin_mut().createCheckpoint(&cxx_path);

    // create information for new database-entry
    let context = &UserContext { 
        user_id: user_id.clone(), 
        project_id: project_id.clone(), 
        is_admin: false, 
        is_project_admin: false 
    };

    // add information of new checkpoint to the database
    // HINT (kitsudaiki): It is intended that the task-uuid is also the checkpoint-uuid, because of easier identification
    match checkpoint_table::add_new_checkpoint(&task_uuid, &task_name, &file_path_str, context) {
        Ok(_) => {},
        Err(e) => {
            error!("{}", e);
            let _ = task_table::set_error_state(&task_uuid, &"Internal error".to_string());
            return;
        }
    }
}

fn handle_checkpoint_restore_task(_: &Uuid, task_info: &mut CheckpointRestoreInfo, link_handle: &Arc<Mutex<ClusterLinkHanle>>) {
    let mut link = link_handle.lock().unwrap();

    let file_path_str: String = task_info.path.to_string_lossy().into();
    cxx::let_cxx_string!(cxx_path = file_path_str);
    link.cluster_link.pin_mut().restoreCheckpoint(&cxx_path);
}

pub fn handle_task(task: Task, link_handle: &Arc<Mutex<ClusterLinkHanle>>) {
    match task.info {
        TaskVariant::Training(mut task_info) => {
            handle_train_task(&task.uuid, &mut task_info, link_handle);
        }, 
        TaskVariant::Request(mut task_info) => {
            handle_request_task(&task.uuid, &mut task_info, link_handle);
        }, 
        TaskVariant::CheckpointSave(mut task_info) => {
            handle_checkpoint_save_task(&task.uuid, &task.name, &task.user_id, &task.project_id, &mut task_info, link_handle);
        }, 
        TaskVariant::CheckpointRestore(mut task_info) => {
            handle_checkpoint_restore_task(&task.uuid, &mut task_info, link_handle);
        }
    }
}

pub struct Cluster {
    #[allow(dead_code)]
    pub uuid: Uuid,
    
    pub queue: Arc<Mutex<TaskQueue>>,
    pub link_handle: Arc<Mutex<ClusterLinkHanle>>,

    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
}

// HINT (kitsudaiki): Workaround to fix the error 
// `*const u8 cannot be sent between threads safely.`, which 
// comes with the `UniquePtr<ffi::HanamiCore>`
unsafe impl Send for ClusterLinkHanle {}

impl Cluster {
    pub fn new(uuid: Uuid, cluster_link: UniquePtr<ffi::ClusterLink>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let queue = Arc::new(Mutex::new(init_task_queue()));
        let queue_clone = Arc::clone(&queue);

        let link_handle = Arc::new(Mutex::new(ClusterLinkHanle{cluster_link: cluster_link}));
        let link_handle_clone = Arc::clone(&link_handle);

        let handle = thread::spawn(move || {
            debug!("Started cluster-thread");
            while running_clone.load(Ordering::Relaxed) {
                // println!("Looping forever");

                // get task fromt he task-queue and prcess the task, otherwise sleep until the next check
                let mut queue_handle = queue_clone.lock().unwrap();
                if let Some(task) = queue_handle.get() {
                    drop(queue_handle);
                    //println!("Popped from front: {:?}", task);

                    handle_task(task, &link_handle_clone);
                } else {
                    drop(queue_handle);
                    thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            debug!("Stopped cluster-thread");
        });

        Cluster {
            uuid: uuid,
            link_handle: link_handle,
            queue: queue, 
            handle: Some(handle),
            running,
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    pub fn add_task(&mut self, task: Task) {
        let mut queue_handle = self.queue.lock().unwrap();
        queue_handle.add(task);
    }

    pub fn request(&mut self, inputs: &HashMap<String, Vec<f32>>, outputs: &mut HashMap<String, Vec<f32>>) -> Result<(), String> {
        let mut link = self.link_handle.lock().unwrap();

        // push input-values form dataset into the backend
        for (hexagon_name, data) in inputs {  
            let data_ptr = data.as_ptr();
            cxx::let_cxx_string!(cxx_name = hexagon_name); 
            unsafe {
                if link.cluster_link.pin_mut().fillInput(&cxx_name, data_ptr, data.len() as u64) == false {
                    let msg = format!("Hexagon with name '{hexagon_name}' not found in cluster.");
                    return Err(msg);
                }
            }
        }

        link.cluster_link.pin_mut().doRequest();

        // get output-values from the backend
        for (hexagon_name, data) in outputs {  
            cxx::let_cxx_string!(cxx_name = hexagon_name); 
            // get size of the hexagon, to resize the buffer for the output-values
            let size: u64 = link.cluster_link.pin_mut().getSize(&cxx_name);
            data.resize(size as usize, 0.0f32);
            // get output from the c++ backend
            let data_ptr: *mut f32 = data.as_mut_ptr();
            unsafe {
                if link.cluster_link.pin_mut().getOutput(&cxx_name, data_ptr, data.len() as u64) == false {
                    let msg = format!("Hexagon with name '{hexagon_name}' not found in cluster.");
                    return Err(msg);
                }
            }
        }

        Ok(())
    }

    pub fn train(&mut self, inputs: &HashMap<String, Vec<f32>>, outputs: &HashMap<String, Vec<f32>>) -> Result<(), String> {
        let mut link = self.link_handle.lock().unwrap();

        // push input-values form dataset into the backend
        for (hexagon_name, data) in inputs {  
            let data_ptr = data.as_ptr();
            cxx::let_cxx_string!(cxx_name = hexagon_name); 
            unsafe {
                if link.cluster_link.pin_mut().fillInput(&cxx_name, data_ptr, data.len() as u64) == false {
                    let msg = format!("Hexagon with name '{hexagon_name}' not found in cluster.");
                    return Err(msg);
                }
            }
        }

        // push output-values form dataset into the backend
        for (hexagon_name, data) in outputs {  
            let data_ptr = data.as_ptr();
            cxx::let_cxx_string!(cxx_name = hexagon_name); 
            unsafe {
                if link.cluster_link.pin_mut().fillExpected(&cxx_name, data_ptr, data.len() as u64) == false {
                    let msg = format!("Hexagon with name '{hexagon_name}' not found in cluster.");
                    return Err(msg);
                }
            }
        }

        link.cluster_link.pin_mut().doTrain();

        Ok(())
    }
}

impl Drop for Cluster {
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}
