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

use ainari_common::secret::Secret;

/// Kind of owner of the secrets, which encrypt the disks within a backup. They belong to the
/// backup and not to the project, so they don't count to the quota of the project, don't block
/// the deletion of the project and can't be deleted by the users.
pub const SECRET_OWNER_BACKUP: &str = "backup";

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct SecretCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(length(min = 8, max = 4096))]
    pub secret_payload: Secret,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct SecretCloneInternalReq {
    pub secret_uuid: Uuid,
    pub owned_by: String,
    pub resource_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct SecretGenerateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct SecretGenerateInternalReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    pub owned_by: String,
    pub resource_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SecretResp {
    pub uuid: Uuid,
    pub name: String,
    pub owned_by: String,
    pub resource_uuid: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
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
    pub owned_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SecretListResp {
    pub secrets: Vec<SecretBasicResp>,
}

/// Secret, like it is exported from omamori into a backup. The payload never leaves the omamori in
/// plain text, but is wrapped with the wrap-key of the backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct OmamoriSecretExport {
    /// UUID of the secret at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    /// kind of owner of the secret, like `user` or `image`
    #[validate(length(min = 1, max = 63))]
    pub owned_by: String,
    /// resource, which owns the secret, if it is not owned by a user
    pub resource_uuid: Option<Uuid>,
    /// base64-encoded payload, which is wrapped with the wrap-key (AES-256-GCM with the UUID of the
    /// secret as additional data)
    pub wrapped_payload: String,
}
