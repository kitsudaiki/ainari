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
    CheckpointCreateTask = 2,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskType::TrainTask => "Train-Task",
            TaskType::RequestTask => "Request-Task",
            TaskType::CheckpointCreateTask => "Checkpoint-Create-Task",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for TaskType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Train-Task" => Ok(TaskType::TrainTask),
            "Request-Task" => Ok(TaskType::RequestTask),
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
    pub dataset_column: String,
    pub hexagon: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskCreateTrainReq {
    pub name: String,
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
pub struct TaskCreateCheckpointSaveReq {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskResp {
    pub uuid: Uuid,
    pub name: String,
    pub task_type: TaskType,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub task_type: TaskType,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskListResp {
    pub tasks: Vec<TaskBasicResp>
}
