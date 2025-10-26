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
use uuid::Uuid;
use validator::Validate;

use ainari_common::secret::Secret;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProxyCreateReq {
    pub port: u16,
    #[validate(length(min = 4, max = 127))]
    pub target_address: String,
    pub cluster_uuid: Uuid,
    #[validate(length(min = 4, max = 127))]
    pub registration_key: Secret,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProxyResp {
    pub uuid: Uuid,
    pub port: u16,
    pub target_address: String,
    pub cluster_uuid: Uuid,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProxyBasicResp {
    pub uuid: Uuid,
    pub port: u16,
    pub target_address: String,
    pub cluster_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProxyListResp {
    pub proxys: Vec<ProxyBasicResp>,
}
