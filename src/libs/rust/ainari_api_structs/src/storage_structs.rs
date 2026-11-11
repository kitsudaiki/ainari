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
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

/// Path of a file or directory within the storage-location of an onsen.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct StoragePathQuery {
    /// path relative to the storage-location, an empty path is the storage-location itself
    #[serde(default)]
    #[validate(length(max = 1024))]
    pub path: String,
}

/// Entry of a directory within the storage-location.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct StorageDirectoryEntry {
    /// name of the entry within the directory
    pub name: String,
    /// true, if the entry is a directory
    pub is_dir: bool,
    /// size in bytes, 0 for directories
    pub size: u64,
}

/// Direct entries of a directory within the storage-location.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct StorageDirectoryResp {
    /// the entries sorted by name
    pub entries: Vec<StorageDirectoryEntry>,
}

/// Size and checksum of a file within the storage-location.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct StorageFileInfoResp {
    /// size of the file in bytes
    pub size: u64,
    /// hex-encoded sha256 of the content of the file
    pub sha256: String,
}

/// Type of the storage-backend of an onsen-host.
#[derive(
    Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageBackendType {
    /// Files and volumes on the local disk of the onsen-host
    #[default]
    Local,
}

impl StorageBackendType {
    /// Value of the backend-type, like it is stored in the database
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageBackendType::Local => "LOCAL",
        }
    }
}

impl std::fmt::Display for StorageBackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for StorageBackendType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "LOCAL" => Ok(StorageBackendType::Local),
            _ => Err(format!("'{value}' is not a valid storage-backend-type")),
        }
    }
}

/// Information about the storage-backend of an onsen-host.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct StorageBackendResp {
    /// type of the storage-backend
    pub backend_type: StorageBackendType,
    /// true, if the backend provides volumes, which the sakura-hosts can attach
    pub supports_volumes: bool,
    /// total capacity of the storage in bytes
    pub total_capacity: u64,
    /// free capacity of the storage in bytes
    pub free_capacity: u64,
}

/// Request to create an empty volume within the storage-backend.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VolumeCreateInternalReq {
    /// name of the volume within the backend
    pub backend_id: Uuid,
    /// size of the volume in bytes, a multiple of 512
    #[validate(range(min = 512))]
    pub size: u64,
}

/// Request to grow a volume within the storage-backend.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VolumeResizeInternalReq {
    /// new size of the volume in bytes, a multiple of 512, which is not smaller than the current
    /// size
    #[validate(range(min = 512))]
    pub size: u64,
}

/// Request to copy a whole volume within the storage-backend.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VolumeCopyInternalReq {
    /// name of the new volume within the backend, which gets the copy
    pub target_backend_id: Uuid,
}

/// State of a volume within the storage-backend.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VolumeState {
    /// The volume is complete and can be used
    Ready,
    /// The volume is the target of a copy, which is still running
    Copying,
    /// The copy into the volume failed. The incomplete volume is removed with this response, so
    /// the volume doesn't exist anymore afterwards.
    Failed,
}

/// Information about a volume within the storage-backend.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct VolumeInfoResp {
    /// name of the volume within the backend
    pub backend_id: Uuid,
    /// state of the volume
    pub state: VolumeState,
    /// size of the volume in bytes
    pub size: u64,
    /// space in bytes, which the volume really uses in the storage
    pub used_size: u64,
}

/// Information, how a sakura-host connects to a volume. The variant depends on the
/// storage-backend of the onsen-host, which stores the volume.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VolumeConnection {
    /// The volume is a file within an NFS-export
    Nfs {
        /// address of the NFS-server
        server: String,
        /// path of the export on the NFS-server
        export: String,
        /// name of the file of the volume within the export
        file_name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_connection_serialization() {
        let connection = VolumeConnection::Nfs {
            server: "10.0.0.5".to_string(),
            export: "/var/lib/ainari/onsen/volumes".to_string(),
            file_name: "f0b5c2c4-5d8e-4f39-9a59-2f2b5c7f6a01.img".to_string(),
        };
        let json = serde_json::to_value(&connection).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "nfs",
                "server": "10.0.0.5",
                "export": "/var/lib/ainari/onsen/volumes",
                "file_name": "f0b5c2c4-5d8e-4f39-9a59-2f2b5c7f6a01.img",
            })
        );
        assert_eq!(
            serde_json::from_value::<VolumeConnection>(json).unwrap(),
            connection
        );

        // an unknown variant is rejected and not misinterpreted
        assert!(
            serde_json::from_value::<VolumeConnection>(serde_json::json!({
                "type": "rbd",
                "server": "10.0.0.5",
                "export": "/",
                "file_name": "x",
            }))
            .is_err()
        );
        // a missing field is rejected
        assert!(
            serde_json::from_value::<VolumeConnection>(serde_json::json!({
                "type": "nfs",
                "server": "10.0.0.5",
            }))
            .is_err()
        );
    }

    #[test]
    fn test_storage_backend_type_roundtrip() {
        // the value in the database and in the api have to be the same
        let backend_type = StorageBackendType::Local;
        let json = serde_json::to_string(&backend_type).unwrap();
        assert_eq!(json, format!("\"{}\"", backend_type.as_str()));
        assert_eq!(
            backend_type.as_str().parse::<StorageBackendType>(),
            Ok(backend_type)
        );
        assert!("CEPH".parse::<StorageBackendType>().is_err());
    }

    #[test]
    fn test_volume_state_serialization() {
        assert_eq!(
            serde_json::to_string(&VolumeState::Copying).unwrap(),
            "\"COPYING\""
        );
        assert_eq!(
            serde_json::from_str::<VolumeState>("\"FAILED\"").unwrap(),
            VolumeState::Failed
        );
    }
}
