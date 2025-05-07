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

use log::debug;
use std::collections::VecDeque;

use super::tasks::Task;

#[derive(Default, Debug)]
pub struct TaskQueue {
    pub queue: VecDeque<Task>,
}

impl TaskQueue {
    pub fn add(&mut self, task: Task) {
        debug!("added task to task-queue");
        self.queue.push_back(task);
    }

    pub fn get(&mut self) -> Option<Task> {
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
    use crate::tasks::{Task, InternalTaskType, TaskVariant, CheckpointSaveInfo};
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn test_add_and_get() {
        let task_queue: Arc<Mutex<TaskQueue>> = Arc::new(Mutex::new(init_task_queue()));
        let mut queue = task_queue.lock().unwrap();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let info1 = CheckpointSaveInfo {
            path: "asdf".to_string().into(),
        };
        let info2 = CheckpointSaveInfo {
            path: "asdf".to_string().into(),
        };

        let task1 = Task {
            uuid: uuid1.clone(),
            task_type: InternalTaskType::TrainTask,
            name: "task1".to_string(),
            user_id: "user0815".to_string(),
            project_id: "project0815".to_string(),
            info: TaskVariant::CheckpointSave(info1),
        };
        let task2 = Task {
            uuid: uuid2.clone(),
            task_type: InternalTaskType::RequestTask,
            name: "task2".to_string(),
            user_id: "user0816".to_string(),
            project_id: "project0816".to_string(),
            info: TaskVariant::CheckpointSave(info2),
        };

        queue.add(task1);
        queue.add(task2);

        let task1 = queue.get().unwrap();
        assert_eq!(task1.uuid, uuid1);
        let task2 = queue.get().unwrap();
        assert_eq!(task2.uuid, uuid2);
    }
}