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
use bytemuck::cast_slice;
use std::thread::{self, JoinHandle};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Write;
use std::time::{Duration, Instant};
use std::collections::HashMap;

use ainari_dataset::dataset_io::{DataSetFileReadHandleV1_0, DataSetFileWriteHandleV1_0};
use ainari_structs::task_structs::TaskState;

use crate::database::task_table;
use crate::database::checkpoint_table;
use crate::api::user_context::UserContext;
use crate::core::cluster_handler::*;
use crate::core::blocks::block_trait::*;

use super::processing::worker_queue::*;
use super::processing::output_buffer::*;
use super::processing::task_queue::{TaskQueue, init_task_queue};
use super::processing::tasks::{Task, TaskVariant, TrainInfo, RequestInfo, CheckpointSaveInfo, CheckpointRestoreInfo};


fn apply_input(
    cluster_uuid: &Uuid, 
    hexagon_name: &String, 
    input_ptr: &[f32], 
    input_size: u64, 
    pos_counter: usize, 
    time_length: u64,
    task_type: &WorkerTaskType) -> Result<(), String>
{
    let cluster_handler = CLUSTER_HANDLER.read().unwrap();
    if let Some(input_block_mutex) = cluster_handler.get_input_block(cluster_uuid, hexagon_name) {
        let mut input_block = input_block_mutex.lock().unwrap();
        input_block.apply_input(input_ptr, input_size as usize, pos_counter, time_length as usize);
        let mut worker_queue = WORKER_QUEUE.lock().unwrap();
        let worker_task = WorkerTask{
            task_type: task_type.clone(),
            block: Arc::clone(&input_block_mutex) as Arc<Mutex<dyn Block>>,
        };
        worker_queue.add(worker_task);
    } else {
        let msg = format!("Input with name {hexagon_name} not found in cluster with uuid {cluster_uuid}");
        return Err(msg);
    }

    Ok(())
}

fn apply_expected(
    cluster_uuid: &Uuid, 
    hexagon_name: &String, 
    input_ptr: &[f32], 
    input_size: u64) -> Result<(), String>
{
    let cluster_handler = CLUSTER_HANDLER.read().unwrap();
    if let Some(output_buffer_mutex) = cluster_handler.get_output_buffer(cluster_uuid, hexagon_name) {
        let mut output_buffer = output_buffer_mutex.lock().unwrap();
        output_buffer.reset_output();
        convert_buffer_to_expected(&mut output_buffer, input_ptr, input_size);
    } else {
        let msg = format!("Input with name {hexagon_name} not found in cluster with uuid {cluster_uuid}");
        return Err(msg);
    }

    Ok(())
}

fn run_train(cluster_uuid: &Uuid, finish_counter_mutex: &Arc<Mutex<FinishCounter>>) -> Result<(), String> {
    for _ in 0..10000000 {
        let finish_counter = finish_counter_mutex.lock().unwrap();
        if finish_counter.counter >= finish_counter.input_compare + finish_counter.output_compare {
            return Ok(());
        }
        drop(finish_counter);
        thread::sleep(std::time::Duration::from_micros(1));
    }

    let msg = format!("Timeout while training cluster with uuid {cluster_uuid}");
    return Err(msg);
}

fn run_process(cluster_uuid: &Uuid, finish_counter_mutex: &Arc<Mutex<FinishCounter>>) -> Result<(), String> {
    for _ in 0..10000000 {
        let finish_counter = finish_counter_mutex.lock().unwrap();
        if finish_counter.counter >= finish_counter.output_compare {
            return Ok(());
        }
        drop(finish_counter);
        thread::sleep(std::time::Duration::from_millis(1));
    }

    let msg = format!("Timeout while requesting cluster with uuid {cluster_uuid}");
    return Err(msg);
}

fn get_input_from_dataset(
    cluster_uuid: &Uuid,
    hexagon_name: &String, 
    file_handle: &mut DataSetFileReadHandleV1_0, 
    cycle_count: u64, 
    time_length: u64,
    task_type: &WorkerTaskType) -> Result<(), String> 
{
    // get input-block
    let cluster_handler = CLUSTER_HANDLER.read().unwrap();
    let input_block_mutex = if let Some(i) = cluster_handler.get_input_block(cluster_uuid, hexagon_name) {
        Arc::clone(&i)
    } else {
        let msg = format!("Input with name {hexagon_name} not found in cluster with uuid {cluster_uuid}");
        return Err(msg);
    };
    drop(cluster_handler);

    let mut input_block = input_block_mutex.lock().unwrap();
    
    // fill input with data from dataset
    let mut pos_counter: usize = 0;
    for time_point in 0..time_length {
        let (input_ptr, input_size) = match file_handle.get_data_from_file(&(cycle_count + time_point)) {
            Ok((input_ptr, input_size)) => (input_ptr, input_size),
            Err(msg) => {
                return Err(msg);
            }
        };

        input_block.apply_input(input_ptr, input_size as usize, pos_counter, time_length as usize);

        pos_counter += input_size as usize;
    }

    // add input-block to worker-queue
    let mut worker_queue = WORKER_QUEUE.lock().unwrap();
    let worker_task = WorkerTask{
        task_type: task_type.clone(),
        block: Arc::clone(&input_block_mutex) as Arc<Mutex<dyn Block>>,
    };
    worker_queue.add(worker_task);    

    Ok(())
} 

fn get_expected_from_dataset(
    cluster_uuid: &Uuid,
    hexagon_name: &String, 
    file_handle: &mut DataSetFileReadHandleV1_0, 
    cycle_count: u64, 
    time_length: u64) -> Result<(), String> 
{
    let (input_ptr, input_size) = match file_handle.get_data_from_file(&(cycle_count + time_length - 1)) {
        Ok((input_ptr, input_size)) => (input_ptr, input_size),
        Err(msg) => {
            return Err(msg);
        }
    };

    let _ = apply_expected(cluster_uuid, hexagon_name, input_ptr, input_size)?;

    Ok(())
} 

fn write_output_into_dataset(cluster_uuid: &Uuid, file_handle: &mut DataSetFileWriteHandleV1_0) -> Result<(), String> 
{
    let cluster_handler = CLUSTER_HANDLER.read().unwrap();

    // get column-description from the dataset
    for (hexagon_name, col_get) in &file_handle.header.columns {  
        let size_output = (col_get.end - col_get.start) as usize;
        let mut output_read = vec![0.0f32; size_output];
    
        if let Some(output_buffer_mutex) = cluster_handler.get_output_buffer(cluster_uuid, hexagon_name) {
            let mut output_buffer = output_buffer_mutex.lock().unwrap();
            convert_output_to_buffer(&mut output_read, &mut output_buffer);
            output_buffer.reset_output();
        } else {
            let msg = format!("Input with name {hexagon_name} not found in cluster with uuid {cluster_uuid}");
            return Err(msg);
        }       
    
        let output_bytes = cast_slice(&output_read);
        let _ = file_handle.target_file.write_all(&output_bytes);
    };

    Ok(())
}

fn handle_train_task(task_uuid: &Uuid, cluster_uuid: &Uuid, task_info: &mut TrainInfo, finish_counter: &Arc<Mutex<FinishCounter>>) {
    // check if task was aborted
    if task_table::is_aborted(&task_uuid) {
        return;
    }

    let task_type = WorkerTaskType::Train;
    let mut prev_timestamp = Instant::now();
    let _ = task_table::update_task_state(&task_uuid, &TaskState::Active);

    for epoch_count in 0..task_info.number_of_epochs {
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

            // reset finsih-counter
            let mut counter = finish_counter.lock().unwrap();
            counter.counter = 0;
            drop(counter);

            // push output-values form dataset into the backend
            for (hexagon_name, file_handle) in &mut task_info.outputs {  
                match get_expected_from_dataset(cluster_uuid, hexagon_name, file_handle, cycle_count, task_info.time_length) {
                    Ok(()) => {},
                    Err(e) => {
                        let _ = task_table::set_error_state(&task_uuid, &e);
                        return;
                    }
                }
            }

            // push input-values form dataset into the backend
            for (hexagon_name, file_handle) in &mut task_info.inputs {  
                match get_input_from_dataset(cluster_uuid, hexagon_name, file_handle, cycle_count, task_info.time_length, &task_type) {
                    Ok(()) => {},
                    Err(e) => {
                        let _ = task_table::set_error_state(&task_uuid, &e);
                        return;
                    }
                }
            }

            match run_train(cluster_uuid, finish_counter) {
                Ok(()) => {},
                Err(e) => {
                    let _ = task_table::set_error_state(&task_uuid, &e);
                    return;
                }
            }
        }
    }

    let cluster_handler = CLUSTER_HANDLER.read().unwrap();
    for (hexagon_name, _) in &mut task_info.outputs {  
        if let Some(output_buffer_mutex) = cluster_handler.get_output_buffer(cluster_uuid, hexagon_name) {
            let mut output_buffer = output_buffer_mutex.lock().unwrap();
            output_buffer.reset_output();
        }
    }

    let _ = task_table::update_task_state(&task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &(task_info.number_of_epochs as i64), &(task_info.number_of_cycles as i64));
}

fn handle_request_task(task_uuid: &Uuid, cluster_uuid: &Uuid, task_info: &mut RequestInfo, finish_counter: &Arc<Mutex<FinishCounter>>) {
    // check if task was aborted
    if task_table::is_aborted(&task_uuid) {
        return;
    }
    
    let task_type = WorkerTaskType::Process;
    let mut prev_timestamp = Instant::now();
    let _ = task_table::update_task_state(&task_uuid, &TaskState::Active);

    for cycle_count in 0..task_info.number_of_cycles {
        // update current state in database at least after 1 second
        let now = Instant::now();
        if now.duration_since(prev_timestamp) >= Duration::from_secs(1) {
            prev_timestamp = now;
            let _ = task_table::update_task_progress(task_uuid, &0 , &(cycle_count as i64));

            // check if task was aborted
            if task_table::is_aborted(&task_uuid) {
                return;
            }
        }

        // reset finsih-counter
        let mut counter = finish_counter.lock().unwrap();
        counter.counter = 0;
        drop(counter);

        // push input-values form dataset into the backend
        for (hexagon_name, file_handle) in &mut task_info.inputs {  
            match get_input_from_dataset(cluster_uuid, hexagon_name, file_handle, cycle_count, task_info.time_length, &task_type) {
                Ok(()) => {},
                Err(e) => {
                    let _ = task_table::set_error_state(&task_uuid, &e);
                    return;
                }
            }
        }
        
        match run_process(cluster_uuid, finish_counter) {
            Ok(()) => {},
            Err(e) => {
                let _ = task_table::set_error_state(&task_uuid, &e);
                return;
            }
        }

        // get output-values form backend and write them into the dataset
        match write_output_into_dataset(cluster_uuid, &mut task_info.results) {
            Ok(()) => {},
            Err(e) => {
                let _ = task_table::set_error_state(&task_uuid, &e);
                return;
            }
        }
    }

    let _ = task_table::update_task_state(&task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &(1 as i64), &(task_info.number_of_cycles as i64));
}

fn handle_checkpoint_save_task(
    cluster_uuid: &Uuid, 
    task_uuid: &Uuid, 
    task_name: &String, 
    user_id: &String, 
    project_id: &String, 
    task_info: &mut CheckpointSaveInfo) 
{
    let cluster_handler = CLUSTER_HANDLER.read().unwrap();
    match cluster_handler.create_checkpoint(cluster_uuid, &task_info.path) {
        Ok(()) => {},
        Err(_) => {
            return;
        },
    }

    // create information for new database-entry
    let context = &UserContext { 
        user_id: user_id.clone(), 
        project_id: project_id.clone(), 
        is_admin: false, 
        is_project_admin: false 
    };

    // add information of new checkpoint to the database
    // HINT (kitsudaiki): It is intended that the task-uuid is also the checkpoint-uuid, because of easier identification
    let file_path_str: String = task_info.path.to_string_lossy().into();
    match checkpoint_table::add_new_checkpoint(&task_uuid, &task_name, &file_path_str, context) {
        Ok(_) => {},
        Err(e) => {
            log::error!("{}", e);
            let _ = task_table::set_error_state(&task_uuid, &"Internal error".to_string());
            return;
        }
    }

    let _ = task_table::update_task_state(&task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &1, &1);
}

fn handle_checkpoint_restore_task(
    cluster_uuid: &Uuid, 
    task_uuid: &Uuid, 
    task_info: &mut CheckpointRestoreInfo) 
{
    let mut cluster_handler = CLUSTER_HANDLER.write().unwrap();
    match cluster_handler.restore_checkpoint(cluster_uuid, &task_info.path) {
        Ok(()) => {},
        Err(_) => {
            return;
        },
    }

    let _ = task_table::update_task_state(&task_uuid, &TaskState::Finished);
    let _ = task_table::update_task_progress(task_uuid, &1, &1);
}

pub fn handle_task(task: Task, finish_counter: &Arc<Mutex<FinishCounter>>) {
    match task.info {
        TaskVariant::Training(mut task_info) => {
            handle_train_task(&task.uuid, &task.cluster_uuid, &mut task_info, finish_counter);
        }, 
        TaskVariant::Request(mut task_info) => {
            handle_request_task(&task.uuid, &task.cluster_uuid, &mut task_info, finish_counter);
        }, 
        TaskVariant::CheckpointSave(mut task_info) => {
            handle_checkpoint_save_task(&task.cluster_uuid, &task.uuid, &task.name, &task.user_id, &task.project_id, &mut task_info);
        }, 
        TaskVariant::CheckpointRestore(mut task_info) => {
            handle_checkpoint_restore_task(&task.cluster_uuid, &task.uuid, &mut task_info);
        }
    }
}

pub struct ClusterInterface {
    pub queue: Arc<Mutex<TaskQueue>>,
    pub finish_counter_mutex: Arc<Mutex<FinishCounter>>,

    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,

    pub cluster_uuid: Uuid,
}

impl ClusterInterface {
    pub fn new(cluster_uuid: &Uuid, finish_counter_mutex: &Arc<Mutex<FinishCounter>>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let queue = Arc::new(Mutex::new(init_task_queue()));
        let queue_clone = Arc::clone(&queue);

        let finfinish_counter_clone = Arc::clone(finish_counter_mutex);

        let handle = thread::spawn(move || {
            log::debug!("Started cluster-thread");
            while running_clone.load(Ordering::Relaxed) {
                // get task fromt he task-queue and prcess the task, otherwise sleep until the next check
                let mut queue_handle = queue_clone.lock().unwrap();
                if let Some(task) = queue_handle.get() {
                    drop(queue_handle);
                    handle_task(task, &finfinish_counter_clone);
                } else {
                    drop(queue_handle);
                    thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            log::debug!("Stopped cluster-thread");
        });

        ClusterInterface {
            finish_counter_mutex: Arc::clone(finish_counter_mutex),
            queue: queue, 
            handle: Some(handle),
            running,
            cluster_uuid: cluster_uuid.clone(),
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
        let mut counter = self.finish_counter_mutex.lock().unwrap();
        counter.counter = 0;
        drop(counter);

        // reset output-values in the backend
        {
            let cluster_data_handler = CLUSTER_HANDLER.read().unwrap();
            for hexagon_name in outputs.keys() {  

                if let Some(output_buffer_mutex) = cluster_data_handler.get_output_buffer(&self.cluster_uuid, &hexagon_name) {
                    let mut output_buffer = output_buffer_mutex.lock().unwrap();
                    output_buffer.reset_output();
                }        
            }
        }

        for (hexagon_name, data) in inputs { 
            apply_input(&self.cluster_uuid, &hexagon_name, data.as_slice(), data.len() as u64, 0, 1, &WorkerTaskType::Process)?;
        }

        run_process(&self.cluster_uuid, &self.finish_counter_mutex)?;


        // get output-values from the backend
        let cluster_data_handler = CLUSTER_HANDLER.read().unwrap();
        for (hexagon_name, data) in outputs.iter_mut() {  

            if let Some(output_buffer_mutex) = cluster_data_handler.get_output_buffer(&self.cluster_uuid, &hexagon_name) {
                let mut output_buffer = output_buffer_mutex.lock().unwrap();
                data.resize(output_buffer.output_neurons.len(), 0.0f32);
                convert_output_to_buffer(data, &mut output_buffer);
            }        
        }

        Ok(())
    }

    pub fn train(&mut self, inputs: &HashMap<String, Vec<f32>>, outputs: &HashMap<String, Vec<f32>>) -> Result<(), String> {
        let mut counter = self.finish_counter_mutex.lock().unwrap();
        counter.counter = 0;
        drop(counter);

        for (hexagon_name, data) in outputs { 
            let _ = apply_expected(&self.cluster_uuid, &hexagon_name, data.as_slice(), data.len() as u64); 
        }

        for (hexagon_name, data) in inputs { 
            apply_input(
                &self.cluster_uuid, 
                &hexagon_name, 
                data.as_slice(), 
                data.len() as u64, 
                0, 
                1, 
                &WorkerTaskType::Train)?;
        }

        run_train(&self.cluster_uuid, &self.finish_counter_mutex)?;

        Ok(())
    }
}

impl Drop for ClusterInterface {
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::core::processing::worker_handler::*;

    use super::*;

    fn run_single_iteration(cluster_uuid: &Uuid, finish_counter_mutex: &Arc<Mutex<FinishCounter>>, input: &[f32;4], expected: &[f32;4]) {
        let input_name = "test_input".to_string();
        let output_name = "test_output".to_string();

        let mut counter = finish_counter_mutex.lock().unwrap();
        counter.counter = 0;
        drop(counter);

        match apply_input(cluster_uuid, &input_name, input, input.len() as u64, 0, 1, &WorkerTaskType::Train) {
            Ok(()) => {},
            Err(e) => {
                println!("{e}");
                assert!(false);
                return;
            },
        }

        match apply_expected(cluster_uuid, &output_name, expected, expected.len() as u64) {
            Ok(()) => {},
            Err(e) => {
                println!("{e}");
                assert!(false);
                return;
            },
        }

        match run_train(cluster_uuid, finish_counter_mutex) {
            Ok(()) => {},
            Err(e) => {
                println!("{e}");
                assert!(false);
                return;
            },
        }
    }

    #[test]
    #[serial]
    fn test_workflow() {
        // Initialize processing
        let worker_handler = WORKER_HANDLER.lock().unwrap();
        drop(worker_handler);
        let cluster_data_handler = CLUSTER_HANDLER.write().unwrap();
        drop(cluster_data_handler);

        // create dummy-cluster
        let cluster_uuid = Uuid::new_v4();
        let cluster_name = "test_cluster".to_string();
        let input1 = [1.0f32, 2.0f32, -3.0f32, 4.0f32];
        let expected1 = [1.0f32, 1.0f32, 0.0f32, 1.0f32];

        let input2 = [5.0f32, -1.0f32, 8.0f32, -4.0f32];
        let expected2 = [0.0f32, 1.0f32, 1.0f32, 0.0f32];

        let template = "version: 1 
        settings:
            neuron_cooldown: 1000000000.0;
            refractory_time: 1;
            max_connection_distance: 1;
        hexagons: 
            1,1,1; 
            2,2,2; 
            3,2,2; 
        axons: 
            1,1,1 -> 2,2,2; 
        inputs: 
            test_input: 1,1,1; 
        outputs: 
            test_output: 3,2,2;".to_string();

        let mut root_handler = CLUSTER_HANDLER.write().unwrap();
        root_handler.clusters.clear();
        let _ = root_handler.init_new_cluster(&cluster_uuid, &cluster_name, template);
        let finish_counter_mutex = root_handler.get_finish_counter(&cluster_uuid).unwrap();
        drop(root_handler);

        for _ in 0..100 {
            run_single_iteration(&cluster_uuid, &finish_counter_mutex, &input1, &expected1);
            run_single_iteration(&cluster_uuid, &finish_counter_mutex, &input2, &expected2);
        }

        println!("finished");
    }
}

