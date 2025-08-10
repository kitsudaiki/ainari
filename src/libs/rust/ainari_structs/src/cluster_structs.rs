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
use std::collections::HashMap;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ClusterCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(length(min = 10))]
    pub template: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterResp {
    pub uuid: Uuid,
    pub name: String,
    pub template: String,
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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ClusterTrainReq {
    #[validate(length(min = 1))]
    pub inputs: HashMap<String, Vec<f32>>,
    #[validate(length(min = 1))]
    pub outputs: HashMap<String, Vec<f32>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ClusterRequestReq {
    #[validate(length(min = 1))]
    pub inputs: HashMap<String, Vec<f32>>,
    #[validate(length(min = 1))]
    pub outputs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ClusterRequestResp {
    pub outputs: HashMap<String, Vec<f32>>,
}

