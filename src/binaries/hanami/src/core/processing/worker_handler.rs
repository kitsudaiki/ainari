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

use std::sync::{Arc, Mutex};

use crate::config;

use ainari_hardware::cpu::*;
use ainari_hardware::memory::*;

use super::worker_thread::WorkerThread;

lazy_static::lazy_static! {
    pub static ref WORKER_HANDLER: Arc<Mutex<WorkerHandler>> = Arc::new(Mutex::new(init_worker_handler()));
}

pub struct WorkerHandler {
    pub worker_threads: Vec<WorkerThread>,
}

pub fn init_worker_handler() -> WorkerHandler {
    let mut worker_handler = WorkerHandler {
        worker_threads: Vec::new(),
    };

    let number_of_threads = match get_number_of_cpu_threads() {
        Ok(number_of_threads) => (number_of_threads as u64) - 2, // reserve 2 threads for other tasks
        Err(e) => {
            let msg = format!("Failed to get number of cpu-threads: {e}");
            log::error!("{msg}");
            panic!("{msg}");
        }
    };

    let use_of_free_memory: f32 = config::CONFIG.processing.use_of_free_memory;
    let free_amount_of_memory = get_free_memory_amount();
    let memory_usage = (free_amount_of_memory as f32 * use_of_free_memory) as u64;

    for i in 0..number_of_threads {
        let new_thread = WorkerThread::new(i);
        worker_handler.worker_threads.push(new_thread);
    }

    log::debug!("Initialized {number_of_threads} cpu-threads for the core.");
    log::debug!("Initialized {memory_usage} byte of memory for the core.");

    worker_handler
}
