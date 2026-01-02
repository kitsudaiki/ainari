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

use std::sync::{Arc, Mutex};

use crate::config;

use ainari_common::constants::*;
use ainari_hardware::cpu::*;

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

    // get number of cpu-threads of the local system
    let mut number_of_threads = match get_number_of_cpu_threads() {
        Ok(number_of_threads) => number_of_threads - NUMBER_OF_RESERVED_THREADS, // reserve NUMBER_OF_RESERVED_THREADS threads for other tasks
        Err(e) => {
            let msg = format!("Failed to get number of cpu-threads: {e}");
            log::error!("{msg}");
            panic!("{msg}");
        }
    };

    // limit number of used threads, if a limit was set
    let max_number_of_threads: usize = config::CONFIG.processing.max_number_of_threads;
    if max_number_of_threads > 0 && number_of_threads > max_number_of_threads {
        number_of_threads = max_number_of_threads;
        log::info!("Limit number of cpu-threads to {max_number_of_threads} based on the config.")
    }

    // initialize new worker-threads
    for i in 0..number_of_threads {
        let new_thread = WorkerThread::new(i);
        worker_handler.worker_threads.push(new_thread);
    }

    log::info!("Initialized {number_of_threads} cpu-threads for the core.");

    worker_handler
}
