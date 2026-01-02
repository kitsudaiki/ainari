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

use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub enum TaskType {
    Train = 0,
    Request = 1,
    CheckpointSave = 2,
    CheckpointRestore = 3,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskType::Train => "TrainTask",
            TaskType::Request => "RequestTask",
            TaskType::CheckpointSave => "CheckpointSaveTask",
            TaskType::CheckpointRestore => "CheckpointRestoreTask",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TaskType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TrainTask" => Ok(TaskType::Train),
            "RequestTask" => Ok(TaskType::Request),
            "CheckpointSaveTask" => Ok(TaskType::CheckpointSave),
            "CheckpointRestoreTask" => Ok(TaskType::CheckpointRestore),
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
        write!(f, "{s}")
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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskDatasetLink {
    pub dataset_uuid: Uuid,
    #[validate(length(min = 4, max = 127))]
    pub dataset_column: String,
    #[validate(length(min = 4, max = 127))]
    pub hexagon: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskDatasetResultLink {
    #[validate(length(min = 4, max = 127))]
    pub hexagon: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskCreateTrainReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(range(min = 1, max = 1000000))]
    pub number_of_epochs: u64,
    #[validate(range(min = 1, max = 100000000))]
    pub time_length: Option<u64>,
    #[validate(nested, length(min = 1))]
    pub inputs: Vec<TaskDatasetLink>,
    #[validate(nested, length(min = 1))]
    pub outputs: Vec<TaskDatasetLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskCreateRequestReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(range(min = 1, max = 100000000))]
    pub time_length: Option<u64>,
    #[validate(nested, length(min = 1))]
    pub inputs: Vec<TaskDatasetLink>,
    #[validate(nested, length(min = 1))]
    pub results: Vec<TaskDatasetResultLink>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskCheckpointSaveReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskCheckpointRestoreReq {
    #[validate(length(min = 4, max = 127))]
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
    pub total_number_of_epochs: i64,
    pub current_epoch: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskListResp {
    pub tasks: Vec<TaskBasicResp>,
}
