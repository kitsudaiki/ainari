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

use core_affinity;
use rand::Rng;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use ainari_common::constants::*;
use ainari_common::error::AinariError;

use super::worker_queue::*;

/// Represents a worker thread that processes tasks from the worker queue.
///
/// The worker thread runs on a specific CPU core and processes tasks until stopped.
/// It handles different types of tasks including training, processing, and backpropagation.
pub struct WorkerThread {
    /// Unique identifier for the thread
    #[allow(dead_code)]
    pub thread_id: usize,

    /// Handle to the spawned thread
    pub handle: Option<JoinHandle<()>>,

    /// Atomic boolean flag to control the thread's running state
    pub running: Arc<AtomicBool>,
}

/// Attempts to finalize the given worker task after a specified number of retries.
///
/// This function tries to finalize the task by calling the appropriate finalization method
/// based on the task type. It retries the operation up to 1000 times with a 1ms delay between attempts.
/// If all attempts fail, it logs an error and returns Ok(()) regardless.
///
/// # Arguments
/// * `worker_task` - A reference to the worker task to be finalized
///
/// # Returns
/// * `Result<(), AinariError>` - Ok(()) if finalization succeeds or after all retries,
///   Err(AinariError) if an error occurs during finalization
fn finalize_task(worker_task: &WorkerTask) -> Result<(), AinariError> {
    // Try to finalize the task up to 1000 times
    for _ in 0..1000 {
        // Acquire the lock on the task's block
        let mut block = worker_task.block.lock().expect("mutex poisoned");

        // Perform the appropriate finalization based on task type
        let success = match worker_task.task_type {
            WorkerTaskType::Train => {
                block.finalize_train(worker_task.cycle_number)?;
                true
            }
            WorkerTaskType::Process => {
                block.finalize_process(worker_task.cycle_number)?;
                true
            }
            WorkerTaskType::Backpropagate => {
                block.finalize_backpropagate(worker_task.cycle_number)?
            }
        };
        // Explicitly drop the lock to allow other threads to access the block
        drop(block);

        // If successful, return immediately
        if success {
            return Ok(());
        }

        // Wait before retrying
        thread::sleep(Duration::from_millis(1));
    }

    // Log an error if all attempts failed
    log::error!("Timeout while try to backpropagate");
    Ok(())
}

/// Processes a worker task according to its type.
///
/// This function handles the execution of the task based on its type (Train, Process, or Backpropagate).
/// It performs the appropriate operation on the task's block and then finalizes the task.
/// If the task requires updating a finish counter, it does so after finalization.
///
/// # Arguments
/// * `worker_task` - A reference to the worker task to be processed
///
/// # Returns
/// * `Result<(), AinariError>` - Ok(()) if processing succeeds, Err(AinariError) if an error occurs
fn process_task(worker_task: &WorkerTask) -> Result<(), AinariError> {
    // Variable to store the optional finish counter mutex
    // Declare outside the scope to allow access after the block is dropped
    #[allow(clippy::needless_late_init)]
    let finish_counter_option;

    // Acquire the lock on the task's block
    {
        let mut block = worker_task.block.lock().expect("mutex poisoned");

        // Perform the appropriate operation based on task type
        match worker_task.task_type {
            WorkerTaskType::Train => {
                // For training tasks, select a random offset within the block dimensions
                let place_offset = rand::rng().random_range(0..BLOCK_DIM);
                finish_counter_option = block.train(
                    place_offset,
                    Arc::clone(&worker_task.block),
                    worker_task.cycle_number,
                )?;
            }
            WorkerTaskType::Process => {
                finish_counter_option = block.process(worker_task.cycle_number)?;
            }
            WorkerTaskType::Backpropagate => {
                finish_counter_option = block.backpropagate(worker_task.cycle_number)?;
            }
        }
    }

    // Finalize the task
    finalize_task(worker_task)?;

    // Update the finish counter if needed
    // HINT (kitsudaiki): This can not be done within the blocks, because it would result in a dead-lock
    //                    when the last input- or output-block tries to trigger the next cycle
    if let Some(finish_counter_mutex) = finish_counter_option {
        let mut finish_counter = finish_counter_mutex.lock().expect("mutex poisoned");
        finish_counter.update(worker_task.cycle_number);
    }

    Ok(())
}

impl WorkerThread {
    /// Creates a new WorkerThread instance.
    ///
    /// This function initializes a new worker thread on the specified CPU core.
    /// The thread will process tasks from the worker queue until it is stopped.
    ///
    /// # Arguments
    /// * `thread_id` - The ID of the CPU core to run the thread on
    ///
    /// # Returns
    /// * `WorkerThread` - A new WorkerThread instance
    pub fn new(thread_id: usize) -> Self {
        // Create an atomic boolean to control the thread's running state
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        log::debug!("Create worker-thread on cpu-thread {thread_id}");

        // Spawn the worker thread
        let handle = thread::spawn(move || {
            log::debug!("Started worker-thread on cpu-thread {thread_id}");

            // Set CPU affinity to pin the thread to the specified core
            let core_id = core_affinity::CoreId { id: thread_id };
            let res = core_affinity::set_for_current(core_id);
            if !res {
                log::warn!("Failed to pin worker-thread to cpu-thread {thread_id}");
            }

            // Main thread loop
            while running_clone.load(Ordering::Relaxed) {
                // Get a task from the worker queue
                let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
                if let Some(worker_task) = worker_queue.get() {
                    drop(worker_queue);

                    // Process the task and handle any errors
                    match process_task(&worker_task) {
                        Ok(()) => {}
                        Err(AinariError::Unauthorized(msg)) => {
                            log::error!("{msg}");
                            // TODO: better error-handling
                        }
                        Err(AinariError::InvalidInput(msg)) => {
                            log::error!("{msg}");
                            // TODO: better error-handling
                        }
                        Err(AinariError::InternalError(msg)) => {
                            log::error!("{msg}");
                            // TODO: better error-handling
                        }
                    };
                } else {
                    drop(worker_queue);
                    // Sleep briefly if there are no tasks to process
                    thread::sleep(Duration::from_millis(1));
                }
            }

            log::debug!("Stopped worker-thread on cpu-thread {thread_id}");
        });

        // Return the new WorkerThread instance
        WorkerThread {
            thread_id,
            handle: Some(handle),
            running,
        }
    }

    /// Stops the worker thread.
    ///
    /// This function sets the running flag to false and waits for the thread to finish execution.
    pub fn stop(&mut self) {
        // Set the running flag to false
        self.running.store(false, Ordering::Relaxed);

        // Wait for the thread to finish
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WorkerThread {
    /// Ensures the worker thread is stopped when the WorkerThread instance is dropped.
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}
