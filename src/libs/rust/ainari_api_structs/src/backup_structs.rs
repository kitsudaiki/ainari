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
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

/// State of the package of a backup, which is stored as one directory on a backup-target. The
/// package is only handled as a whole, so its state defines, which operations are allowed on its
/// files.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupStatus {
    /// The backup is created. Files can only be written into the package in this state.
    Creating,
    /// The package is complete and sealed, so it is read only from now on
    Ready,
    /// The package is deleted as a whole. A failed deletion stays in this state and can be repeated.
    Deleting,
    /// The package is incomplete or inconsistent and can only be deleted
    Error,
}

impl BackupStatus {
    /// Value of the status, like it is stored in the database
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupStatus::Creating => "CREATING",
            BackupStatus::Ready => "READY",
            BackupStatus::Deleting => "DELETING",
            BackupStatus::Error => "ERROR",
        }
    }
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BackupStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "CREATING" => Ok(BackupStatus::Creating),
            "READY" => Ok(BackupStatus::Ready),
            "DELETING" => Ok(BackupStatus::Deleting),
            "ERROR" => Ok(BackupStatus::Error),
            _ => Err(format!("'{value}' is not a valid backup-status")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupCreateReq {
    /// UUID of the backup, which is also the name of its directory on the backup-target
    pub uuid: Uuid,
    /// project, which the backup belongs to
    #[validate(length(min = 1, max = 255))]
    pub project_id: String,
    /// backup-target, where the backup is stored. Without it, ryokan selects one.
    pub host_uuid: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupListQuery {
    /// Project to list only its backups. Without it, all backups are listed, which are visible
    /// for the user.
    pub project_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupResp {
    pub uuid: Uuid,
    /// backup-target, which stores the package
    pub host_uuid: Uuid,
    /// address of the onsen of the backup-target
    pub onsen_address: String,
    /// path of the package relative to the storage-location of the onsen
    pub directory_path: String,
    pub backup_status: BackupStatus,
    /// number of files in the package, known after it was finalized
    pub file_count: u64,
    /// size of all files of the package in bytes, known after it was finalized
    pub total_size: u64,
    /// hex-encoded sha256 of the manifest of the package, known after it was finalized
    pub manifest_sha256: Option<String>,
    pub owner_id: String,
    pub project_id: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupListResp {
    pub backups: Vec<BackupResp>,
}

/// Maximum size in bytes of a single file, which is written into a package or read from it over
/// the api. It is meant for the metadata and the manifest of a backup, while large files, like
/// disks, are stored directly in the onsen.
pub const MAX_BACKUP_FILE_SIZE: usize = 16 * 1024 * 1024;

/// Maximum size in bytes of the json-message, which transfers a file of a package. The
/// base64-encoded content is a third larger than the file itself, plus some space for the other
/// fields of the message.
pub const MAX_BACKUP_FILE_MESSAGE_SIZE: usize = MAX_BACKUP_FILE_SIZE.div_ceil(3) * 4 + 4096;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupFileQuery {
    /// path of the file relative to the directory of the package
    #[validate(length(min = 1, max = 1024))]
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupFileWriteReq {
    /// path of the file relative to the directory of the package
    #[validate(length(min = 1, max = 1024))]
    pub path: String,
    /// base64-encoded content of the file
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupFileResp {
    /// path of the file relative to the directory of the package
    pub path: String,
    /// base64-encoded content of the file
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupDiskReq {
    /// virtual_machine, whose root-disk is stored in the package
    pub vm_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupDiskResp {
    /// address of the onsen of the backup-target, where the disk is uploaded to
    pub onsen_address: String,
    /// path of the disk relative to the storage-location of the onsen
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupScanEntry {
    /// UUID of the backup, which is the name of its directory
    pub uuid: Uuid,
    /// true, if the package contains a manifest. A package without manifest is incomplete.
    pub has_manifest: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupScanResp {
    /// packages on the backup-target, which are not registered as backup
    pub backups: Vec<BackupScanEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupRegisterReq {
    /// UUID of the backup, which is the name of its directory on the backup-target
    pub uuid: Uuid,
    /// project, which the backup belongs to
    #[validate(length(min = 1, max = 255))]
    pub project_id: String,
    /// backup-target, where the package is stored
    pub host_uuid: Uuid,
}

/// Direction of a copy between the package of a backup and the normal onsens.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupCopyDirection {
    /// The file of an image is copied into the package, which must be in the state `CREATING`
    IntoBackup,
    /// A file of the package is copied to a normal onsen, where it can be registered as image.
    /// The package must be in the state `READY`.
    OutOfBackup,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupCopyReq {
    pub direction: BackupCopyDirection,
    /// path of the file relative to the directory of the package
    #[validate(length(min = 1, max = 1024))]
    pub path: String,
    /// `INTO_BACKUP`: image, whose file is copied into the package. `OUT_OF_BACKUP`: new UUID of
    /// the image, under which the file is stored on the normal onsen.
    pub image_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupCopyResp {
    /// address of the onsen, where the file was copied to
    pub onsen_address: String,
    /// path of the copy relative to the storage-location of the onsen
    pub file_path: String,
    /// size of the copied file in bytes
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_status_round_trip() {
        for status in [
            BackupStatus::Creating,
            BackupStatus::Ready,
            BackupStatus::Deleting,
            BackupStatus::Error,
        ] {
            assert_eq!(status.as_str().parse::<BackupStatus>(), Ok(status));
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", status.as_str()));
        }
        assert!("UNKNOWN".parse::<BackupStatus>().is_err());
    }

    #[test]
    fn test_max_backup_file_message_size() {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;

        // a file with the maximum size fits into the json-message
        let resp = BackupFileResp {
            path: "a".repeat(1024),
            content: BASE64.encode(vec![0u8; MAX_BACKUP_FILE_SIZE]),
        };
        let message = serde_json::to_string(&resp).unwrap();
        assert!(message.len() <= MAX_BACKUP_FILE_MESSAGE_SIZE);
    }

    #[test]
    fn test_backup_copy_direction() {
        let json = serde_json::to_string(&BackupCopyDirection::OutOfBackup).unwrap();
        assert_eq!(json, "\"OUT_OF_BACKUP\"");
        let direction: BackupCopyDirection = serde_json::from_str("\"INTO_BACKUP\"").unwrap();
        assert_eq!(direction, BackupCopyDirection::IntoBackup);
    }
}
