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

use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use hanami_dataset::dataset_io::{DataSetFileReadHandleV1_0, DataSetFileWriteHandleV1_0};

use crate::api::http_endpoints::cluster::task::task_structs::{TaskState, TaskType};

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
    pub task_type: TaskType,
    pub name: String,
    pub user_id: String,
    pub project_id: String,

    pub info: TaskVariant,
}
