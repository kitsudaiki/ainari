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

use std::time::SystemTime;
use uuid::Uuid;
use std::collections::HashMap;
use serde::Deserialize;

use hanami_dataset::dataset_io::DataSetFileHandle;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum TaskType {
    NoTask = 0,
    TrainTask = 1,
    RequestTask = 2,
    ClusterCheckpointSaveTask = 3,
    ClusterCheckpointRestoreTask = 4,
}

#[derive(Debug)]
pub enum TaskState {
    UndefinedTaskState = 0,
    QueuedTaskState = 1,
    ActiveTaskState = 2,
    AbortedTaskState = 3,
    FinishedTaskState = 4,
}

pub struct TaskProgress {
    pub task_state: TaskState,
    pub total_number_of_cycles: u64,
    pub current_cycle: u64,
    pub queued_time_stamp: SystemTime,
    pub start_active_time_stamp: SystemTime,
    pub end_active_time_stamp: SystemTime,
    pub estimated_remaining_time: u64,
}

#[derive(Debug)]
pub struct TrainInfo {
    pub inputs: HashMap<String, DataSetFileHandle>,
    pub outputs: HashMap<String, DataSetFileHandle>,

    pub number_of_cycles: u64,
    pub current_cycle: u64,
    pub time_length: u64,
}

#[derive(Debug)]
pub struct RequestInfo {
    pub inputs: HashMap<String, DataSetFileHandle>,
    pub results: HashMap<String, DataSetFileHandle>,

    pub number_of_cycles: u64,
    pub current_cycle: u64,
    pub time_length: u64,
}

#[derive(Debug)]
pub struct CheckpointSaveInfo {
    pub path: String,
}

#[derive(Debug)]
pub struct CheckpointRestoreInfo {
    pub path: String,
}

#[derive(Debug)]
enum TaskVariant {
    Training(TrainInfo),
    Request(RequestInfo),
    CheckpointSave(CheckpointSaveInfo),
    CheckpointRestore(CheckpointRestoreInfo),
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Task {
    pub uuid: Uuid,
    pub task_type: TaskType,
    pub name: String,
    pub userId: String,
    pub projectId: String,

    //pub info: TaskVariant,
}

// struct WorkerTask {
//     Cluster* cluster = nullptr;
//     uint32_t hexagonId = UNINIT_STATE_32;
//     blockId = UNINIT_STATE_32;
//     ClusterProcessingMode mode = ClusterProcessingMode::TRAIN_BACKWARD_MODE;
// };
