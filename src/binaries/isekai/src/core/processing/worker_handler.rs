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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use crate::config;

use super::jobs::Job;
use super::worker_thread::WorkerThread;

lazy_static::lazy_static! {
    pub static ref WORKER_HANDLER: Arc<Mutex<WorkerHandler>> = Arc::new(Mutex::new(init_worker_handler()));
}

pub struct WorkerHandler {
    /// Vector containing all worker threads managed by this handler.
    pub worker_threads: Vec<WorkerThread>,
}

/// Adds a job to the queue of its worker-thread.
///
/// All jobs of the same project are processed by the same worker-thread, so they run one after
/// another, like the lock of the project requires it anyway.
///
/// # Arguments
///
/// * `job` - The job
pub fn add_job_to_queue(job: Job) {
    let mut worker_handler = WORKER_HANDLER.lock().expect("mutex poisoned");
    let worker_id = worker_index(&job.project_id, worker_handler.worker_threads.len());

    if let Some(worker) = worker_handler.worker_threads.get_mut(worker_id) {
        worker.add_job(job);
    }
}

/// Selects the worker-thread of a project.
///
/// # Arguments
///
/// * `project_id` - The project
/// * `number_of_workers` - Number of worker-threads
///
/// # Returns
///
/// The index of the worker-thread.
fn worker_index(project_id: &str, number_of_workers: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    project_id.hash(&mut hasher);
    (hasher.finish() % number_of_workers.max(1) as u64) as usize
}

/// Starts the worker-threads, which process the jobs.
///
/// # Returns
///
/// The handler of the worker-threads.
pub fn init_worker_handler() -> WorkerHandler {
    let number_of_workers = config::CONFIG.processing.number_of_workers.max(1);
    let worker_threads = (0..number_of_workers)
        .map(|_| WorkerThread::new())
        .collect();
    log::info!("Initialized {number_of_workers} worker-threads for the jobs.");

    WorkerHandler { worker_threads }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_index() {
        for workers in 1..8 {
            let index = worker_index("project", workers);
            assert!(index < workers);
            // the same project always lands on the same worker
            assert_eq!(index, worker_index("project", workers));
        }
        assert_eq!(worker_index("project", 0), 0);
    }
}
