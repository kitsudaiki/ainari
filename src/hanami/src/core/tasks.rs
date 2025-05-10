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
use std::collections::HashMap;
use std::path::PathBuf;
use serde::Deserialize;
use uuid::Uuid;

use hanami_dataset::dataset_io::{DataSetFileReadHandleV1_0, DataSetFileWriteHandleV1_0};

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum InternalTaskType {
    TrainTask = 0,
    RequestTask = 1,
    ClusterCheckpointSaveTask = 2,
    ClusterCheckpointRestoreTask = 3,
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
    pub inputs: HashMap<String, DataSetFileReadHandleV1_0>,
    pub outputs: HashMap<String, DataSetFileReadHandleV1_0>,

    pub number_of_cycles: u64,
    pub time_length: u64,
}

#[derive(Debug)]
pub struct RequestInfo {
    pub inputs: HashMap<String, DataSetFileReadHandleV1_0>,
    pub results: HashMap<String, DataSetFileWriteHandleV1_0>,

    pub number_of_cycles: u64,
    pub time_length: u64,
}

#[derive(Debug)]
pub struct CheckpointSaveInfo {
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct CheckpointRestoreInfo {
    pub path: PathBuf,
}

#[derive(Debug)]
pub enum TaskVariant {
    Training(TrainInfo),
    Request(RequestInfo),
    CheckpointSave(CheckpointSaveInfo),
    CheckpointRestore(CheckpointRestoreInfo),
}

#[derive(Debug)]
pub struct Task {
    pub uuid: Uuid,
    pub task_type: InternalTaskType,
    pub name: String,
    pub user_id: String,
    pub project_id: String,

    pub info: TaskVariant,
}
