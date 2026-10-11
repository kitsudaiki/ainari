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
pub struct ImageInitReq {
    pub uuid: Uuid,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    pub image_type: String,
    /// True, if the image is a snapshot of the root-disk of a virtual_machine
    #[serde(default)]
    pub is_snapshot: bool,
    /// Already existing secret in the omamori, which is used to encrypt the image. Without it, a
    /// new secret is generated for the image.
    #[serde(default)]
    pub secret_uuid: Option<Uuid>,
}

/// Fields of the multipart-form, which are uploaded together with the file of a new image
#[derive(Serialize, Deserialize, Debug, Clone, Default, JsonSchema, ApiComponent, Validate)]
pub struct ImageUploadReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    /// Type of the image. At the moment only `disk` is supported.
    pub image_type: String,
    /// Already existing secret in the omamori, which is used to encrypt the image. Without it, a
    /// new secret is generated for the image.
    #[serde(default)]
    pub secret_uuid: Option<Uuid>,
}

/// Type and size of the file of an image, which are measured before the file is encrypted
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ImageFileInfoReq {
    /// Type of the unencrypted file (e.g. "qcow2")
    #[validate(length(min = 1, max = 32))]
    pub file_type: String,
    /// Size of the unencrypted file in bytes
    pub file_size: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ImageInternalResp {
    pub uuid: Uuid,
    pub name: String,
    pub onsen_address: String,
    pub file_path: String,
    /// Type of the unencrypted file (e.g. "qcow2")
    pub file_type: String,
    /// Size of the unencrypted file in bytes
    pub file_size: u64,
    pub is_snapshot: bool,
    pub secret_uuid: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ImageResp {
    pub uuid: Uuid,
    pub name: String,
    /// Type of the unencrypted file (e.g. "qcow2")
    pub file_type: String,
    /// Size of the unencrypted file in bytes
    pub file_size: u64,
    pub is_snapshot: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ImageBasicResp {
    pub uuid: Uuid,
    pub name: String,
    /// Type of the unencrypted file (e.g. "qcow2")
    pub file_type: String,
    /// Size of the unencrypted file in bytes
    pub file_size: u64,
    pub is_snapshot: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ImageListResp {
    pub images: Vec<ImageBasicResp>,
}
