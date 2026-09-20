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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

use ainari_common::error::AinariError;

use super::task_queue::*;
use super::tasks::*;

/// Represents a worker thread that processes tasks from the worker queue.
///
/// The worker thread runs on a specific CPU core and processes tasks until stopped.
/// It handles different types of tasks including training, processing, and backpropagation.
pub struct WorkerThread {
    #[allow(dead_code)]
    pub queue: Arc<Mutex<TaskQueue>>,
    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
}

impl WorkerThread {
    /// Creates a new WorkerThread.
    ///
    /// The thread is started right away and processes tasks from its own queue until it is
    /// stopped again, which happens when the `WorkerThread` is dropped.
    ///
    /// # Returns
    /// * `WorkerThread` - A new WorkerThread
    pub fn new() -> Self {
        log::info!("Create Worker-Thread.");

        // Create an atomic boolean to control the thread's running state
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let queue = Arc::new(Mutex::new(init_task_queue()));
        let queue_clone = Arc::clone(&queue);

        // Spawn the worker thread
        let handle = thread::spawn(move || {
            log::info!("Started Worker-Thread.");

            // Build a single-threaded Tokio runtime specifically for this OS thread
            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime");
            let local = LocalSet::new();

            // Run the loop inside the async runtime
            local.block_on(&rt, async move {
                while running_clone.load(Ordering::Relaxed) {
                    // Get a task from the worker queue
                    let next_task = queue_clone.lock().expect("mutex poisoned").get();
                    if let Some(task_mutex) = next_task {
                        // The queue handed over its only reference to the task, so the task
                        // can be moved out of its mutex. That way no lock is held across the
                        // await-points of the processing.
                        let mut task = match Arc::try_unwrap(task_mutex) {
                            Ok(task_mutex) => task_mutex.into_inner().expect("mutex poisoned"),
                            Err(_) => {
                                log::error!("Task is still referenced elsewhere, skip processing");
                                continue;
                            }
                        };

                        // Process the task and handle any errors
                        match process_task(&mut task).await {
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
                        // Sleep briefly if there are no tasks to process
                        thread::sleep(Duration::from_millis(1));
                    }
                }

                log::info!("Stopped Worker-Thread.");
            });
        });

        // Return the new WorkerThread
        WorkerThread {
            queue,
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

    /// Adds a task to the thread's task queue.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to be added to the queue
    pub fn add_task(&mut self, task: Task) {
        let mut queue_handle = self.queue.lock().expect("mutex poisoned");
        queue_handle.add(task);
    }
}

impl Drop for WorkerThread {
    /// Ensures the worker thread is stopped when the WorkerThread is dropped.
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}
