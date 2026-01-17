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

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::super::blocks::block_trait::*;

lazy_static::lazy_static! {
    /// Global worker queue instance wrapped in `Arc<Mutex<>>` for thread-safe access.
    /// This provides a shared queue that can be accessed by multiple worker threads.
    pub static ref WORKER_QUEUE: Arc<Mutex<WorkerQueue>> = Arc::new(Mutex::new(init_worker_queue()));
}

/// Represents the different types of tasks that can be assigned to workers.
/// Each variant corresponds to a different operation in the neural network pipeline.
#[derive(Clone, PartialEq)]
pub enum WorkerTaskType {
    /// Task for training the neural network.
    Train,
    /// Task for processing input data.
    Process,
    /// Task for performing backpropagation.
    Backpropagate,
}

/// Initializes and returns a new empty `WorkerQueue`.
///
/// This creates a queue structure ready to receive tasks from the main thread
/// and distribute them to worker threads.
pub fn init_worker_queue() -> WorkerQueue {
    WorkerQueue {
        queue: VecDeque::new(),
    }
}

/// Represents a task to be executed by a worker thread.
/// Contains information about the task type, cycle number, and the block to operate on.
pub struct WorkerTask {
    /// The cycle number this task belongs to, useful for ordering and synchronization.
    pub cycle_number: u64,
    /// The type of task to be performed.
    pub task_type: WorkerTaskType,
    /// The block that this task will operate on, wrapped in `Arc<Mutex<>>` for thread-safe access.
    pub block: Arc<Mutex<dyn Block>>,
}

/// A thread-safe queue for managing worker tasks.
/// Uses a `VecDeque` internally for efficient push/pop operations from both ends.
pub struct WorkerQueue {
    /// The underlying queue storing `WorkerTask` instances.
    pub queue: VecDeque<WorkerTask>,
}

impl WorkerQueue {
    /// Adds a new task to the end of the queue.
    ///
    /// # Arguments
    /// * `task` - The task to be added to the queue.
    ///
    /// This operation is O(1) in time complexity.
    pub fn add(&mut self, task: WorkerTask) {
        self.queue.push_back(task);
    }

    /// Retrieves and removes the next task from the front of the queue.
    ///
    /// # Returns
    /// * `Option<WorkerTask>` - Some task if one exists, None if the queue is empty.
    ///
    /// This operation is O(1) in time complexity.
    pub fn get(&mut self) -> Option<WorkerTask> {
        self.queue.pop_front()
    }
}
