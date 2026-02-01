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

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use uuid::Uuid;

use ainari_common::error::AinariError;

use crate::core::model_handler::*;
use crate::core::processing::finish_counter::FinishCounter;

use super::processing::output_buffer::*;
use super::processing::task_queue::{TaskQueue, init_task_queue};
use super::processing::tasks::{self, Task, TaskVariant};
use super::processing::worker_queue::*;

/// Represents an interface to interact with a neural network model.
///
/// This struct manages the execution of tasks on a model, including processing
/// inputs and outputs, training, and monitoring task completion.
pub struct ModelInterface {
    /// Shared task queue containing tasks to be executed by the model
    pub queue: Arc<Mutex<TaskQueue>>,

    /// Shared counter tracking task completion status
    pub finish_counter_mutex: Arc<Mutex<FinishCounter>>,

    /// Thread handle for the model's worker thread
    pub handle: Option<JoinHandle<()>>,

    /// Atomic flag indicating if the model is running
    pub running: Arc<AtomicBool>,

    /// Unique identifier for the model
    pub model_uuid: Uuid,
}

impl ModelInterface {
    /// Creates a new ModelInterface instance.
    ///
    /// # Arguments
    ///
    /// * `model_uuid` - Unique identifier for the model
    /// * `finish_counter_mutex` - Shared counter for tracking task completion
    ///
    /// # Returns
    ///
    /// A new ModelInterface instance with a running worker thread
    pub fn new(model_uuid: &Uuid, finish_counter_mutex: &Arc<Mutex<FinishCounter>>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let queue = Arc::new(Mutex::new(init_task_queue()));
        let queue_clone = Arc::clone(&queue);

        let finish_counter_clone = Arc::clone(finish_counter_mutex);

        let handle = thread::spawn(move || {
            log::debug!("Started model-thread");
            while running_clone.load(Ordering::Relaxed) {
                // get task from the task-queue and process the task, otherwise sleep until the next check
                let mut queue_handle = queue_clone.lock().expect("mutex poisoned");
                if let Some(task_mutex) = queue_handle.get() {
                    drop(queue_handle);

                    // prepare task
                    let wait_for_finish;
                    {
                        let mut task = task_mutex.lock().expect("mutex poisoned");
                        {
                            let mut finish_counter =
                                finish_counter_clone.lock().expect("mutex poisoned");
                            // Reset the finish counter differently for training vs processing tasks
                            if matches!(task.info, TaskVariant::Training(_)) {
                                // For training tasks, we need to compare both input and output tasks
                                let task_compare =
                                    finish_counter.input_compare + finish_counter.output_compare;
                                finish_counter.reset(task_compare, 0);
                            } else {
                                // For processing tasks, we only need to compare output tasks
                                let task_compare = finish_counter.output_compare;
                                finish_counter.reset(task_compare, 0);
                            }
                            finish_counter.task = Some(task_mutex.clone());
                        }

                        // Start the task and determine if we need to wait for completion
                        wait_for_finish = task.start_task();
                    }

                    // wait until task is finished if needed
                    if wait_for_finish {
                        for _ in 0..10000000 {
                            let mut task = task_mutex.lock().expect("mutex poisoned");
                            if task.is_task_finished() {
                                task.finalize_task();
                                break;
                            }
                            drop(task);
                            thread::sleep(std::time::Duration::from_millis(10));
                        }
                    } else {
                        // If no waiting is needed, just finalize the task
                        let mut task = task_mutex.lock().expect("mutex poisoned");
                        task.finalize_task();
                    }
                } else {
                    drop(queue_handle);
                    // No tasks available, sleep for a second before checking again
                    thread::sleep(std::time::Duration::from_secs(1));
                }
            }
            log::debug!("Stopped model-thread");
        });

        ModelInterface {
            finish_counter_mutex: Arc::clone(finish_counter_mutex),
            queue,
            handle: Some(handle),
            running,
            model_uuid: *model_uuid,
        }
    }

    /// Stops the model's worker thread.
    ///
    /// This method sets the running flag to false and joins the worker thread.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Adds a task to the model's task queue.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to be added to the queue
    pub fn add_task(&mut self, task: Task) {
        let mut queue_handle = self.queue.lock().expect("mutex poisoned");
        queue_handle.add(task);
    }

    /// Gets the number of open tasks in the queue.
    ///
    /// # Returns
    ///
    /// The number of tasks currently in the queue
    pub fn get_number_open_tasks(&mut self) -> usize {
        let queue_handle = self.queue.lock().expect("mutex poisoned");
        queue_handle.len()
    }

    /// Processes inputs through the model and returns outputs.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Map of input names to their corresponding data
    /// * `outputs` - Map of output names to buffers that will be filled with results
    ///
    /// # Returns
    ///
    /// Result indicating success or failure of the operation
    pub fn request(
        &mut self,
        inputs: &HashMap<String, Vec<f32>>,
        outputs: &mut HashMap<String, Vec<f32>>,
    ) -> Result<(), AinariError> {
        let mut counter = self.finish_counter_mutex.lock().expect("mutex poisoned");
        let task_compare = counter.output_compare;
        counter.reset(task_compare, 0);
        drop(counter);

        // reset output-values in the backend
        {
            let model_data_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");
            for hexagon_name in outputs.keys() {
                let output_buffer_mutex =
                    model_data_handler.get_output_buffer(&self.model_uuid, hexagon_name)?;
                let mut output_buffer = output_buffer_mutex.lock().expect("mutex poisoned");
                output_buffer.reset_output();
            }
        }

        for (hexagon_name, data) in inputs {
            tasks::apply_plain_input(
                &self.model_uuid,
                hexagon_name,
                data.as_slice(),
                data.len() as u64,
                0,
                1,
                &WorkerTaskType::Process,
            )?;
        }

        run_iteration(&self.model_uuid, &self.finish_counter_mutex)?;

        // get output-values from the backend
        let model_data_handler = CLUSTER_HANDLER.read().expect("mutex poisoned");
        for (hexagon_name, data) in outputs.iter_mut() {
            let output_buffer_mutex =
                model_data_handler.get_output_buffer(&self.model_uuid, hexagon_name)?;

            let mut output_buffer = output_buffer_mutex.lock().expect("mutex poisoned");
            convert_output_to_buffer(data, &mut output_buffer);
        }

        Ok(())
    }

    /// Trains the model using the provided inputs and expected outputs.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Map of input names to their corresponding data
    /// * `outputs` - Map of output names to their expected values
    ///
    /// # Returns
    ///
    /// Result indicating success or failure of the operation
    pub fn train(
        &mut self,
        inputs: &HashMap<String, Vec<f32>>,
        outputs: &HashMap<String, Vec<f32>>,
    ) -> Result<(), AinariError> {
        let mut counter = self.finish_counter_mutex.lock().expect("mutex poisoned");
        let task_compare = counter.input_compare + counter.output_compare;
        counter.reset(task_compare, 0);
        drop(counter);

        for (hexagon_name, data) in outputs {
            let _ = tasks::apply_expected(
                &self.model_uuid,
                hexagon_name,
                data.as_slice(),
                data.len() as u64,
            );
        }

        for (hexagon_name, data) in inputs {
            tasks::apply_plain_input(
                &self.model_uuid,
                hexagon_name,
                data.as_slice(),
                data.len() as u64,
                0,
                1,
                &WorkerTaskType::Train,
            )?;
        }

        run_iteration(&self.model_uuid, &self.finish_counter_mutex)?;

        Ok(())
    }
}

impl Drop for ModelInterface {
    /// Cleanup when the ModelInterface is dropped.
    ///
    /// Ensures the worker thread is stopped before the ModelInterface is destroyed.
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}

/// Executes a single iteration of model processing.
///
/// This function waits for all tasks to complete or times out after a certain number of iterations.
///
/// # Arguments
///
/// * `model_uuid` - Unique identifier for the model
/// * `finish_counter_mutex` - Shared counter for tracking task completion
///
/// # Returns
///
/// Result indicating success or failure of the operation
fn run_iteration(
    model_uuid: &Uuid,
    finish_counter_mutex: &Arc<Mutex<FinishCounter>>,
) -> Result<(), AinariError> {
    for _ in 0..10000000 {
        let finish_counter = finish_counter_mutex.lock().expect("mutex poisoned");
        if finish_counter.is_finished() {
            return Ok(());
        }
        drop(finish_counter);
        thread::sleep(std::time::Duration::from_micros(1));
    }

    let msg = format!("Timeout while processing model with uuid {model_uuid}");
    Err(AinariError::InternalError(msg))
}

#[cfg(test)]
mod tests {
    use ainari_model_parser::model_parser::parse_model_template;
    use serial_test::serial;

    use crate::core::processing::worker_handler::*;

    use super::*;

    fn run_single_iteration(
        model_uuid: &Uuid,
        finish_counter_mutex: &Arc<Mutex<FinishCounter>>,
        input: &[f32; 4],
        expected: &[f32; 4],
    ) {
        let input_name = "test_input".to_string();
        let output_name = "test_output".to_string();

        let mut counter = finish_counter_mutex.lock().expect("mutex poisoned");
        let task_compare = counter.input_compare + counter.output_compare;
        counter.reset(task_compare, 0);
        drop(counter);

        match tasks::apply_plain_input(
            model_uuid,
            &input_name,
            input,
            input.len() as u64,
            0,
            1,
            &WorkerTaskType::Train,
        ) {
            Ok(()) => {}
            Err(e) => {
                println!("{e}");
                panic!();
            }
        }

        match tasks::apply_expected(model_uuid, &output_name, expected, expected.len() as u64) {
            Ok(()) => {}
            Err(e) => {
                println!("{e}");
                panic!();
            }
        }

        match run_iteration(model_uuid, finish_counter_mutex) {
            Ok(()) => {}
            Err(e) => {
                println!("{e}");
                panic!();
            }
        }
    }

    #[test]
    #[serial]
    fn test_workflow() {
        // Initialize processing
        let worker_handler = WORKER_HANDLER.lock().expect("mutex poisoned");
        drop(worker_handler);
        let model_data_handler = CLUSTER_HANDLER.write().expect("mutex poisoned");
        drop(model_data_handler);

        // create dummy-model
        let model_uuid = Uuid::new_v4();
        let model_name = "test_model".to_string();
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
            test_output: 3,2,2;"
            .to_string();

        let mut root_handler = CLUSTER_HANDLER.write().expect("mutex poisoned");
        root_handler.models.clear();
        let mut parsed_model = parse_model_template(&model_name, &template).unwrap();
        parsed_model.uuid = model_uuid;
        let _ = root_handler.init_new_model(&model_uuid, &parsed_model);
        let finish_counter_mutex = root_handler.get_finish_counter(&model_uuid).unwrap();
        drop(root_handler);

        for _ in 0..100 {
            run_single_iteration(&model_uuid, &finish_counter_mutex, &input1, &expected1);
            run_single_iteration(&model_uuid, &finish_counter_mutex, &input2, &expected2);
        }

        println!("finished");
    }
}
