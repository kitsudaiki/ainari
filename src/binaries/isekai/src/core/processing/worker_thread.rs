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

use super::job_queue::JobQueue;
use super::jobs::{Job, process_job};

/// Thread, which processes the jobs of its own queue one after another.
pub struct WorkerThread {
    pub queue: Arc<Mutex<JobQueue>>,
    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
}

impl WorkerThread {
    /// Creates a new WorkerThread.
    ///
    /// The thread is started right away and processes jobs from its own queue until it is
    /// stopped again, which happens when the `WorkerThread` is dropped.
    ///
    /// # Returns
    /// * `WorkerThread` - A new WorkerThread
    pub fn new() -> Self {
        log::info!("Create Worker-Thread.");

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let queue = Arc::new(Mutex::new(JobQueue::default()));
        let queue_clone = Arc::clone(&queue);

        let handle = thread::spawn(move || {
            log::info!("Started Worker-Thread.");

            // Build a single-threaded Tokio runtime specifically for this OS thread, because the
            // clients of the other components are not `Send`
            let rt = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime");
            let local = LocalSet::new();

            local.block_on(&rt, async move {
                while running_clone.load(Ordering::Relaxed) {
                    let next_job = queue_clone.lock().expect("mutex poisoned").get();
                    match next_job {
                        Some(job) => process_job(job).await,
                        // Sleep briefly if there are no jobs to process
                        None => tokio::time::sleep(Duration::from_millis(100)).await,
                    }
                }

                log::info!("Stopped Worker-Thread.");
            });
        });

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
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Adds a job to the thread's job queue.
    ///
    /// # Arguments
    ///
    /// * `job` - The job to be added to the queue
    pub fn add_job(&mut self, job: Job) {
        self.queue.lock().expect("mutex poisoned").add(job);
    }
}

impl Drop for WorkerThread {
    /// Ensures the worker thread is stopped when the WorkerThread is dropped.
    fn drop(&mut self) {
        self.stop();
    }
}
