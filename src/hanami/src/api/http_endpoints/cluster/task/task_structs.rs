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

use apistos::ApiComponent;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use std::str::FromStr;
use std::fmt;


#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub enum TaskType {
    TrainTask = 0,
    RequestTask = 1,
    CheckpointSaveTask = 2,
    CheckpointRestoreTask = 3,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskType::TrainTask => "TrainTask",
            TaskType::RequestTask => "RequestTask",
            TaskType::CheckpointSaveTask => "CheckpointSaveTask",
            TaskType::CheckpointRestoreTask => "CheckpointRestoreTask",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for TaskType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TrainTask" => Ok(TaskType::TrainTask),
            "RequestTask" => Ok(TaskType::RequestTask),
            "CheckpointSaveTask" => Ok(TaskType::CheckpointSaveTask),
            "CheckpointRestoreTask" => Ok(TaskType::CheckpointRestoreTask),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub enum TaskState {
    Created = 0,
    Queued = 1,
    Active = 2,
    Aborted = 3,
    Finished = 4,
    Error = 5,
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskState::Created => "CREATED",
            TaskState::Queued => "QUEUED",
            TaskState::Active => "ACTIVE",
            TaskState::Aborted => "ABORTED",
            TaskState::Finished => "FINISHED",
            TaskState::Error => "ERROR",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for TaskState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CREATED" => Ok(TaskState::Created),
            "QUEUED" => Ok(TaskState::Queued),
            "ACTIVE" => Ok(TaskState::Active),
            "ABORTED" => Ok(TaskState::Aborted),
            "FINISHED" => Ok(TaskState::Finished),
            "ERROR" => Ok(TaskState::Error),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskDatasetLink {
    pub dataset_uuid: Uuid,
    pub dataset_column: String,
    pub hexagon: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskDatasetResultLink {
    pub hexagon: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskCreateTrainReq {
    pub name: String,
    pub number_of_epochs: u64,
    pub inputs: Vec<TaskDatasetLink>,
    pub outputs: Vec<TaskDatasetLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskCreateRequestReq {
    pub name: String,
    pub inputs: Vec<TaskDatasetLink>,
    pub results: Vec<TaskDatasetResultLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskCheckpointSaveReq {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskCheckpointRestoreReq {
    pub name: String,
    pub checkpoint_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskResp {
    pub uuid: Uuid,
    pub name: String,
    pub task_type: TaskType,
    pub state: TaskState,
    pub total_number_of_epochs: i64,
    pub current_epoch: i64,
    pub total_number_of_cycles: i64,
    pub current_cycle: i64,
    pub queued_at: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub created_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub task_type: TaskType,
    pub state: TaskState,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskListResp {
    pub tasks: Vec<TaskBasicResp>
}
