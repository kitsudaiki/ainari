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

use serde::Deserialize;
use log::{info, debug, error};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

use super::tasks::{Task, TaskType};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TaskQueue {
    pub queue: VecDeque<Task>,
}

impl TaskQueue {
    fn add(&mut self, task: Task) {
        self.queue.push_back(task);
    }

    fn get(&mut self) -> Option<Task> {
        self.queue.pop_front()
    }
}

pub fn init_task_queue() -> TaskQueue {
    let task_queue = TaskQueue {
        queue: VecDeque::new(),
    };
    task_queue
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn test_add_and_get() {
        let task_queue: Arc<Mutex<TaskQueue>> = Arc::new(Mutex::new(init_task_queue()));
        let mut queue = task_queue.lock().unwrap();

        let task1 = Task {
            uuid: Uuid::new_v4(),
            task_type: TaskType::TrainTask,
            name: "task1".to_string(),
            userId: "user0815".to_string(),
            projectId: "project0815".to_string(),
        };
        let task2 = Task {
            uuid: Uuid::new_v4(),
            task_type: TaskType::RequestTask,
            name: "task2".to_string(),
            userId: "user0816".to_string(),
            projectId: "project0816".to_string(),
        };

        queue.add(task1.clone());
        queue.add(task2.clone());

        assert_eq!(queue.get(), Some(task1));
        assert_eq!(queue.get(), Some(task2));
    }
}