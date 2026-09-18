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
pub struct PublicKeyUploadReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    /// The ssh-public-key in its one-line openssh-representation. The fingerprint is not provided
    /// here, but calculated from this key by the server.
    #[validate(length(min = 8, max = 4096))]
    pub public_key: String,
}

/// Public representation of a public-key, which only contains the fingerprint of the key.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct PublicKeyResp {
    pub uuid: Uuid,
    pub name: String,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct PublicKeyBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct PublicKeyListResp {
    pub public_keys: Vec<PublicKeyBasicResp>,
}

/// Internal representation of a public-key for other components of the backend, which also
/// contains the key itself.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct PublicKeyInternalResp {
    pub uuid: Uuid,
    pub name: String,
    pub public_key: String,
    pub fingerprint: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}
