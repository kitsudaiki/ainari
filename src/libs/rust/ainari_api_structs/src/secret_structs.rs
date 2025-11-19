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
pub struct SecretCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(length(min = 8, max = 4096))]
    pub secret_payload: Secret,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct SecretGenerateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SecretResp {
    pub uuid: Uuid,
    pub name: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SecretWithPayloadResp {
    pub secret_payload: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SecretBasicResp {
    pub uuid: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SecretListResp {
    pub secrets: Vec<SecretBasicResp>,
}
