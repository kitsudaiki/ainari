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

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::super::blocks::block_trait::*;

lazy_static::lazy_static! {
    pub static ref WORKER_QUEUE: Arc<Mutex<WorkerQueue>> = Arc::new(Mutex::new(init_worker_queue()));
}

#[derive(Clone)]
pub enum WorkerTaskType {
    Train,
    Process,
    Backpropagate,
}

pub fn init_worker_queue() -> WorkerQueue {
    WorkerQueue {
        queue: VecDeque::new(),
    }
}

pub struct WorkerTask {
    pub task_type: WorkerTaskType,
    pub block: Arc<Mutex<dyn Block>>,
}

pub struct WorkerQueue {
    pub queue: VecDeque<WorkerTask>,
}

impl WorkerQueue {
    pub fn add(&mut self, task: WorkerTask) {
        self.queue.push_back(task);
    }

    pub fn get(&mut self) -> Option<WorkerTask> {
        self.queue.pop_front()
    }
}
