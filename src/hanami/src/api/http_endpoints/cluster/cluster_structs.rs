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
use std::collections::HashMap;


#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, PartialEq)]
pub enum ClusterMode {
    Task,
    Direct,
}

impl fmt::Display for ClusterMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ClusterMode::Task => "Task",
            ClusterMode::Direct => "Direct",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for ClusterMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Task" => Ok(ClusterMode::Task),
            "Direct" => Ok(ClusterMode::Direct),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterCreateReq {
    pub name: String,
    pub template: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterModeSetReq {
    pub mode: ClusterMode,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterResp {
    pub uuid: Uuid,
    pub name: String,
    pub template: String,
    pub mode: ClusterMode,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterBasicResp {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterListResp {
    pub clusters: Vec<ClusterBasicResp>
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterTrainReq {
    pub inputs: HashMap<String, Vec<f32>>,
    pub outputs: HashMap<String, Vec<f32>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterRequestReq {
    pub inputs: HashMap<String, Vec<f32>>,
    pub outputs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterRequestResp {
    pub outputs: HashMap<String, Vec<f32>>,
}

