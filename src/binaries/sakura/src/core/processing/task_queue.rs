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
    #[allow(dead_code)]
    pub fn get_number_open_tasks(&self) -> usize {
        self.queue.len()
    }

    /// Removed all remaining entries from the queue
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.queue.clear();
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
    use crate::core::processing::tasks::{
        CloudHypervisorVirtualMachineCreateInfo, Task, TaskMeta, TaskVariant,
    };
    use ainari_api_structs::user_context::UserContext;

    fn test_context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        }
    }

    #[test]
    fn test_add_and_get() {
        let virtual_machine_uuid = Uuid::new_v4();
        let task_queue: Arc<Mutex<TaskQueue>> = Arc::new(Mutex::new(init_task_queue()));
        let mut queue = task_queue.lock().expect("mutex poisoned");
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let _secret = Secret::from("asdf");
        let resource_type = TaskResourceType::VirtualMachine;

        let info1 = CloudHypervisorVirtualMachineCreateInfo {
            vm_uuid: virtual_machine_uuid,
            name: "test-task1".to_string(),
            context: test_context(),
        };
        let info2 = CloudHypervisorVirtualMachineCreateInfo {
            vm_uuid: virtual_machine_uuid,
            name: "test-task2".to_string(),
            context: test_context(),
        };

        let task1 = Task {
            uuid: uuid1,
            resouce_uuid: virtual_machine_uuid,
            resource_type: resource_type.clone(),
            name: "task1".to_string(),
            info: TaskVariant::CloudHypervisorVirtualMachineCreate(info1),
            meta: TaskMeta::new(),
        };
        let task2 = Task {
            uuid: uuid2,
            resouce_uuid: virtual_machine_uuid,
            resource_type: resource_type.clone(),
            name: "task2".to_string(),
            info: TaskVariant::CloudHypervisorVirtualMachineCreate(info2),
            meta: TaskMeta::new(),
        };

        queue.add(task1);
        queue.add(task2);

        // let queue_len = queue.len();
        // assert_eq!(queue_len, 2);

        let task1 = queue.get().unwrap();
        assert_eq!(task1.lock().expect("mutex poisoned").uuid, uuid1);
        let task2 = queue.get().unwrap();
        assert_eq!(task2.lock().expect("mutex poisoned").uuid, uuid2);
    }
}
