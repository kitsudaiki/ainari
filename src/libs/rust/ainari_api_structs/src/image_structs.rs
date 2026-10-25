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
use std::collections::BTreeMap;
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

use crate::project_structs::ImportedResource;

/// State of the file of an image. Only an image with the state `READY` can be used for a
/// virtual_machine.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImageStatus {
    /// The encrypted file is stored in the onsen, by ryokan for an uploaded image or by sakura for a
    /// snapshot
    Uploading,
    /// The file is encrypted and stored in the onsen
    Encrypting,
    /// The image is complete and can be used
    Ready,
    /// Reserved for images, which are used by a virtual_machine. It is not set at the moment.
    InUse,
}

impl ImageStatus {
    /// Value of the status, like it is stored in the database
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageStatus::Uploading => "UPLOADING",
            ImageStatus::Encrypting => "ENCRYPTING",
            ImageStatus::Ready => "READY",
            ImageStatus::InUse => "IN_USE",
        }
    }
}

impl std::fmt::Display for ImageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ImageStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "UPLOADING" => Ok(ImageStatus::Uploading),
            "ENCRYPTING" => Ok(ImageStatus::Encrypting),
            "READY" => Ok(ImageStatus::Ready),
            "IN_USE" => Ok(ImageStatus::InUse),
            _ => Err(format!("'{value}' is not a valid image-status")),
        }
    }
}

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

/// New status of an image, optionally together with type and size of its file, which are measured
/// before the file is encrypted
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ImageUpdateInternalReq {
    pub image_status: ImageStatus,
    /// Type of the unencrypted file (e.g. "qcow2"). Must be provided together with `file_size`.
    #[validate(length(min = 1, max = 32))]
    #[serde(default)]
    pub file_type: Option<String>,
    /// Size of the unencrypted file in bytes. Must be provided together with `file_type`.
    #[serde(default)]
    pub file_size: Option<u64>,
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
    pub image_status: ImageStatus,
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
    pub image_status: ImageStatus,
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
    pub image_status: ImageStatus,
    pub is_snapshot: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ImageListResp {
    pub images: Vec<ImageBasicResp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_status_roundtrip() {
        for image_status in [
            ImageStatus::Uploading,
            ImageStatus::Encrypting,
            ImageStatus::Ready,
            ImageStatus::InUse,
        ] {
            // the value in the database and in the api have to be the same
            let json = serde_json::to_string(&image_status).unwrap();
            assert_eq!(json, format!("\"{}\"", image_status.as_str()));
            assert_eq!(
                image_status.as_str().parse::<ImageStatus>(),
                Ok(image_status)
            );
        }
        assert!("ACTIVE".parse::<ImageStatus>().is_err());
    }
}

/// Image or snapshot, like it is exported from ryokan into a backup. The location of its file is
/// not part of it, because the file is stored within the backup itself.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct RyokanImageExport {
    /// UUID of the image at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    /// type of the unencrypted file (e.g. "qcow2")
    #[validate(length(min = 1, max = 31))]
    pub file_type: String,
    /// size of the unencrypted file in bytes
    pub file_size: u64,
    pub is_snapshot: bool,
    /// secret, which encrypts the file of the image
    pub secret_uuid: Uuid,
}

/// Image from a backup, whose file was already copied to a normal onsen.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RyokanImageImport {
    #[validate(nested)]
    pub image: RyokanImageExport,
    /// address of the onsen, where the file of the image was copied to
    #[validate(length(min = 1, max = 255))]
    pub onsen_address: String,
    /// path of the file on the onsen, which must be within `images/`
    #[validate(length(min = 1, max = 1024))]
    pub file_path: String,
}

/// Images and snapshots of a project, like they are exported from ryokan into a backup.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RyokanProjectExport {
    #[validate(nested)]
    pub images: Vec<RyokanImageExport>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RyokanProjectImportReq {
    #[validate(nested)]
    pub images: Vec<RyokanImageImport>,
    /// overwrite existing images of the project with the same UUID
    #[serde(default)]
    pub force: bool,
    /// new UUIDs of the resources, whose UUID was already taken. It is applied to the UUIDs of the
    /// images and to their secrets. UUIDs, which are not part of it, are kept.
    #[serde(default)]
    pub id_map: BTreeMap<Uuid, Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct RyokanProjectImportResp {
    pub project_id: String,
    pub images: Vec<ImportedResource>,
}
