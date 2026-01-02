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

pub struct WorkerThread {
    #[allow(dead_code)]
    pub thread_id: usize,

    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
}

fn finalize_task(worker_task: &WorkerTask) -> Result<(), AinariError> {
    for _ in 0..1000 {
        let mut block = worker_task.block.lock().expect("mutex poisoned");
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
        drop(block);

        if success {
            return Ok(());
        }

        thread::sleep(Duration::from_millis(1));
    }

    log::error!("Timeout while try to backpropagate");
    Ok(())
}

fn process_task(worker_task: &WorkerTask) -> Result<(), AinariError> {
    #[allow(clippy::needless_late_init)]
    let finish_counter_option;
    {
        let mut block = worker_task.block.lock().expect("mutex poisoned");
        match worker_task.task_type {
            WorkerTaskType::Train => {
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

    finalize_task(worker_task)?;

    // register the backpropagation for the input-bock to update the finish-counter
    // HINT (kitsudaiki): This can not be done within the blocks, because it would result in a dead-lock
    //                    when the last input- or output-block tries to trigger the next cycle
    if let Some(finish_counter_mutex) = finish_counter_option {
        let mut finish_counter = finish_counter_mutex.lock().expect("mutex poisoned");
        finish_counter.update(worker_task.cycle_number);
    }

    Ok(())
}

impl WorkerThread {
    pub fn new(thread_id: usize) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        log::debug!("Create worker-thread on cpu-thread {thread_id}");

        let handle = thread::spawn(move || {
            log::debug!("Started worker-thread on cpu-thread {thread_id}");
            let core_id = core_affinity::CoreId { id: thread_id };
            let res = core_affinity::set_for_current(core_id);
            if !res {
                log::warn!("Failed to pin worker-thread to cpu-thread {thread_id}");
            }
            while running_clone.load(Ordering::Relaxed) {
                let mut worker_queue = WORKER_QUEUE.lock().expect("mutex poisoned");
                if let Some(worker_task) = worker_queue.get() {
                    drop(worker_queue);
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
                        Err(AinariError::Error(msg)) => {
                            log::error!("{msg}");
                            // TODO: better error-handling
                        }
                    };
                } else {
                    drop(worker_queue);
                    thread::sleep(Duration::from_millis(1));
                }
            }
            log::debug!("Stopped worker-thread on cpu-thread {thread_id}");
        });

        WorkerThread {
            thread_id,
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
}

impl Drop for WorkerThread {
    fn drop(&mut self) {
        self.stop(); // make sure to stop thread on drop~!
    }
}
