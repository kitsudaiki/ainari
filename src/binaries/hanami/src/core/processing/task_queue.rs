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

use crate::database::task_table;
use ainari_api_structs::task_structs::*;

use super::tasks::Task;

#[derive(Default, Debug)]
pub struct TaskQueue {
    pub queue: VecDeque<Arc<Mutex<Task>>>,
}

impl TaskQueue {
    pub fn add(&mut self, task: Task) {
        log::debug!("added task to task-queue");
        let _ = task_table::update_task_state(&task.uuid, &TaskState::Queued);
        self.queue.push_back(Arc::new(Mutex::new(task)));
    }

    pub fn get(&mut self) -> Option<Arc<Mutex<Task>>> {
        self.queue.pop_front()
    }
}

pub fn init_task_queue() -> TaskQueue {
    TaskQueue {
        queue: VecDeque::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use uuid::Uuid;

    use super::*;
    use crate::core::processing::tasks::{CheckpointSaveInfo, Task, TaskMeta, TaskVariant};

    #[test]
    fn test_add_and_get() {
        let cluster_uuid = Uuid::new_v4();
        let task_queue: Arc<Mutex<TaskQueue>> = Arc::new(Mutex::new(init_task_queue()));
        let mut queue = task_queue.lock().expect("mutex poisoned");
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let info1 = CheckpointSaveInfo {
            path: "asdf".to_string().into(),
        };
        let info2 = CheckpointSaveInfo {
            path: "asdf".to_string().into(),
        };

        let task1 = Task {
            uuid: uuid1,
            cluster_uuid,
            name: "task1".to_string(),
            token: "user0815".to_string(),
            info: TaskVariant::CheckpointSave(info1),
            meta: TaskMeta::new(1, 1, 1),
        };
        let task2 = Task {
            uuid: uuid2,
            cluster_uuid,
            name: "task2".to_string(),
            token: "user0816".to_string(),
            info: TaskVariant::CheckpointSave(info2),
            meta: TaskMeta::new(1, 1, 1),
        };

        queue.add(task1);
        queue.add(task2);

        let task1 = queue.get().unwrap();
        assert_eq!(task1.lock().expect("mutex poisoned").uuid, uuid1);
        let task2 = queue.get().unwrap();
        assert_eq!(task2.lock().expect("mutex poisoned").uuid, uuid2);
    }
}
