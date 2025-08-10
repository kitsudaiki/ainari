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

use std::thread::{self, JoinHandle};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use rand::Rng;

use ainari_common::constants::*;

use super::worker_queue::*;

pub struct WorkerThread {
    #[allow(dead_code)]
    pub thread_id: u64,

    pub handle: Option<JoinHandle<()>>,
    pub running: Arc<AtomicBool>,
}

fn process_task(task: &WorkerTask) {
    let mut block = task.block.lock().unwrap();
    match task.task_type {
        WorkerTaskType::Train => {
            let random_pos = rand::rng().random_range(0..BLOCK_DIM);
            block.train(random_pos, Arc::clone(&task.block));
        }
        WorkerTaskType::Process => {
            block.process();
        }
        WorkerTaskType::Backpropagate => {
            block.backpropagate();
        }
    }
}

impl WorkerThread {
    pub fn new(thread_id: u64) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);


        let handle = thread::spawn(move || {
            log::debug!("Started worker-thread");
            while running_clone.load(Ordering::Relaxed) {
                let mut worker_queue = WORKER_QUEUE.lock().unwrap();
                if let Some(task) = worker_queue.get() {
                    drop(worker_queue);
                    process_task(&task);
                } else {
                    drop(worker_queue);
                    thread::sleep(Duration::from_millis(1));
                }
            }
            log::debug!("Stopped worker-thread");
        });

        WorkerThread {
            thread_id: thread_id,
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
