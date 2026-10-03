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
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VmTypeCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(range(min = 1))]
    pub number_of_cores: i32,
    /// memory of the vm-type in MiB
    #[validate(range(min = 1))]
    pub amount_of_memory: i64,
}

/// Only the values, which are set, are updated.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VmTypeUpdateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: Option<String>,
    #[validate(range(min = 1))]
    pub number_of_cores: Option<i32>,
    /// memory of the vm-type in MiB
    #[validate(range(min = 1))]
    pub amount_of_memory: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VmTypeResp {
    pub uuid: Uuid,
    pub name: String,
    pub number_of_cores: i32,
    /// memory of the vm-type in MiB
    pub amount_of_memory: i64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VmTypeBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub number_of_cores: i32,
    /// memory of the vm-type in MiB
    pub amount_of_memory: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VmTypeListResp {
    pub vm_types: Vec<VmTypeBasicResp>,
}
