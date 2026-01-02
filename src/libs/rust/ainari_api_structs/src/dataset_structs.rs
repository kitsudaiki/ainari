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
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct DatasetInitReq {
    pub uuid: Uuid,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    pub dataset_type: String,
    pub number_of_rows: u64,
    pub column_names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct DatasetInternalResp {
    pub uuid: Uuid,
    pub name: String,
    pub onsen_address: String,
    pub file_path: String,
    pub number_of_rows: u64,
    pub column_names: Vec<String>,
    pub secret_uuid: Uuid,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct DatasetResp {
    pub uuid: Uuid,
    pub name: String,
    pub number_of_rows: u64,
    pub column_names: Vec<String>,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct DatasetBasicResp {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct DatasetListResp {
    pub datasets: Vec<DatasetBasicResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct DatasetCheckReq {
    #[validate(length(min = 4, max = 127))]
    pub dataset_column: String,
    pub reference_uuid: Uuid,
    #[validate(length(min = 4, max = 127))]
    pub reference_column: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct DatasetCheckResp {
    pub accuracy: f32,
}
