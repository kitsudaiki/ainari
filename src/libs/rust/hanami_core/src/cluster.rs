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

use log::{info, debug, error};
use uuid::Uuid;
use std::collections::HashMap;

use crate::task_queue::TaskQueue;

use super::task_queue;
use super::tasks;

#[derive(Debug, Clone, PartialEq)]
enum ClusterMode {
    TaskMode,
    DirectMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub version: u8,
    pub uuid: Uuid,
    pub name: String,

    pub mode: ClusterMode,

    queue: TaskQueue,
}

impl Default for Cluster {
    fn default() -> Self { 
        Cluster { 
            version: 1,
            uuid: Uuid::new_v4(),
            name: "".to_string(),
            mode: ClusterMode::TaskMode,
            queue: task_queue::init_task_queue(),
        }
    }
}
