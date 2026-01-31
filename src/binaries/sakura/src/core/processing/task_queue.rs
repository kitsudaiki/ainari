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

use crate::database::task_table;
use ainari_api_structs::task_structs::*;

use super::tasks::Task;

/// A thread-safe queue for managing tasks in the system.
///
/// This structure maintains a queue of tasks wrapped in `Arc<Mutex<Task>>` to allow
/// for shared ownership and thread-safe access. The queue follows FIFO (First-In-First-Out)
/// ordering for task processing.
#[derive(Default, Debug)]
pub struct TaskQueue {
    /// The underlying queue storing tasks wrapped in `Arc<Mutex<Task>>`.
    /// The `VecDeque` provides efficient push/pop operations from both ends.
    pub queue: VecDeque<Arc<Mutex<Task>>>,
}

impl TaskQueue {
    /// Adds a new task to the end of the queue.
    ///
    /// This method takes ownership of the task, wraps it in an `Arc<Mutex<>>` for
    /// thread-safe shared access, and adds it to the queue. It also updates the
    /// task's state in the database to `Queued`.
    ///
    /// # Arguments
    ///
    /// * `task` - The task to be added to the queue
    pub fn add(&mut self, task: Task) {
        log::debug!("added task to task-queue");
        // Update task state in database before adding to queue
        let _ = task_table::update_task_state(&task.uuid, &TaskState::Queued);
        // Wrap task in Arc<Mutex<>> for thread-safe shared access
        self.queue.push_back(Arc::new(Mutex::new(task)));
    }

    /// Removes and returns the task at the front of the queue.
    ///
    /// This method retrieves the next task to be processed from the queue without
    /// taking ownership of the queue itself. Returns `None` if the queue is empty.
    ///
    /// # Returns
    ///
    /// * `Option<Arc<Mutex<Task>>>` - The task at the front of the queue, or `None` if empty
    pub fn get(&mut self) -> Option<Arc<Mutex<Task>>> {
        self.queue.pop_front()
    }

    /// Returns the number of tasks currently in the queue.
    ///
    /// This method provides a quick way to check the current size of the queue
    /// without modifying it.
    ///
    /// # Returns
    ///
    /// * `usize` - The number of tasks in the queue
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

/// Initializes a new empty task queue.
///
/// This function creates and returns a new `TaskQueue` with an empty internal queue.
/// The returned queue is ready to have tasks added to it.
///
/// # Returns
///
/// * `TaskQueue` - A new empty task queue
pub fn init_task_queue() -> TaskQueue {
    TaskQueue {
        queue: VecDeque::new(),
    }
}
#[cfg(test)]
mod tests {
    use ainari_common::secret::Secret;
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use super::*;
    use crate::core::processing::tasks::{CheckpointSaveInfo, Task, TaskMeta, TaskVariant};

    #[test]
    fn test_add_and_get() {
        let model_uuid = Uuid::new_v4();
        let task_queue: Arc<Mutex<TaskQueue>> = Arc::new(Mutex::new(init_task_queue()));
        let mut queue = task_queue.lock().expect("mutex poisoned");
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let secret = Secret::from("asdf");

        let info1 = CheckpointSaveInfo {
            onsen_address: "127.0.0.1".to_string(),
            file_path: "asdf".to_string(),
            secret: secret.clone(),
        };
        let info2 = CheckpointSaveInfo {
            onsen_address: "127.0.0.1".to_string(),
            file_path: "asdf".to_string(),
            secret: secret.clone(),
        };

        let task1 = Task {
            uuid: uuid1,
            model_uuid,
            name: "task1".to_string(),
            info: TaskVariant::CheckpointSave(info1),
            meta: TaskMeta::new(1, 1, 1, 0),
        };
        let task2 = Task {
            uuid: uuid2,
            model_uuid,
            name: "task2".to_string(),
            info: TaskVariant::CheckpointSave(info2),
            meta: TaskMeta::new(1, 1, 1, 0),
        };

        queue.add(task1);
        queue.add(task2);

        let queue_len = queue.len();
        assert_eq!(queue_len, 2);

        let task1 = queue.get().unwrap();
        assert_eq!(task1.lock().expect("mutex poisoned").uuid, uuid1);
        let task2 = queue.get().unwrap();
        assert_eq!(task2.lock().expect("mutex poisoned").uuid, uuid2);
    }
}
