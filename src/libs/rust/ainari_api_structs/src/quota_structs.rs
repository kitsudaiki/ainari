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
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct QuotaResp {
    pub user_id: String,
    pub max_cluster: i32,
    pub max_dataset: i32,
    pub max_checkpoint: i32,
    pub max_secret: i32,
    pub max_taskqueue: i32,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct QuotaSetReq {
    pub max_cluster: i32,
    pub max_dataset: i32,
    pub max_checkpoint: i32,
    pub max_secret: i32,
    pub max_taskqueue: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct QuotaBasicResp {
    pub user_id: String,
    pub max_cluster: i32,
    pub max_dataset: i32,
    pub max_checkpoint: i32,
    pub max_secret: i32,
    pub max_taskqueue: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct QuotaListResp {
    pub quotas: Vec<QuotaBasicResp>,
}
