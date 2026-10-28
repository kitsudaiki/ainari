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

//! Layout of the directory of a backup and the content of its metadata-files.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use ainari_api_structs::exports::{ImageExport, SakuraVirtualMachineExport};
use ainari_common::encryption::{decrypt_bytes, encrypt_bytes};
use ainari_common::error::AinariError;

use super::keys::BackupKeys;

/// Version of the content of the metadata-files, which are written by this version of isekai
pub const METADATA_SCHEMA_VERSION: u32 = 1;

/// Metadata of miko: the project and its quota
pub const MIKO_METADATA: &str = "metadata/miko.json.enc";
/// Metadata of omamori: the secrets and public-keys
pub const OMAMORI_METADATA: &str = "metadata/omamori.json.enc";
/// Metadata of ryokan: the images and snapshots
pub const RYOKAN_METADATA: &str = "metadata/ryokan.json.enc";
/// Metadata of hanami: the networks, virtual_machines, packet-filters and floating ip-addresses
pub const HANAMI_METADATA: &str = "metadata/hanami.json.enc";
/// Metadata of sakura: the config of the virtual_machines and their root-disks
pub const SAKURA_METADATA: &str = "metadata/sakura.json.enc";

/// Path of the root-disk of a virtual_machine within a backup.
///
/// # Arguments
///
/// * `virtual_machine_uuid` - The virtual_machine
///
/// # Returns
///
/// The path relative to the directory of the backup.
pub fn disk_path(virtual_machine_uuid: &Uuid) -> String {
    format!("disks/{virtual_machine_uuid}.enc")
}

/// Path of the file of an image within a backup.
///
/// # Arguments
///
/// * `image_uuid` - The image
///
/// # Returns
///
/// The path relative to the directory of the backup.
pub fn image_path(image_uuid: &Uuid) -> String {
    format!("images/{image_uuid}.enc")
}

/// Content of a metadata-file, which is versioned, so an older backup can be migrated, when it
/// is read again.
#[derive(Serialize, Deserialize)]
struct MetadataFile<T> {
    schema_version: u32,
    data: T,
}

/// Root-disk of a virtual_machine within a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BackupDisk {
    /// path of the disk relative to the directory of the backup
    pub path: String,
    /// secret in omamori, which encrypts the disk. Its payload is part of the metadata of omamori.
    pub secret_uuid: Uuid,
}

/// Virtual_machine within the metadata of sakura.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BackupVirtualMachine {
    pub config: SakuraVirtualMachineExport,
    pub disk: BackupDisk,
}

/// Content of the metadata-file of sakura.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SakuraMetadata {
    pub virtual_machines: Vec<BackupVirtualMachine>,
}

/// Content of the metadata-file of ryokan.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RyokanMetadata {
    pub images: Vec<ImageExport>,
    /// true, if the files of the images are part of the backup under `images/`
    pub files_included: bool,
}

/// Encrypts the content of a metadata-file.
///
/// The file is bound to its path within the backup by the additional data of the encryption, so
/// two metadata-files can't be swapped.
///
/// # Arguments
///
/// * `path` - Path of the file relative to the directory of the backup
/// * `data` - Content of the file
/// * `keys` - Keys of the backup
///
/// # Returns
///
/// The encrypted file.
pub fn encrypt_metadata<T: Serialize>(
    path: &str,
    data: &T,
    keys: &BackupKeys,
) -> Result<Vec<u8>, AinariError> {
    let file = MetadataFile {
        schema_version: METADATA_SCHEMA_VERSION,
        data,
    };
    let plain = zeroize::Zeroizing::new(
        serde_json::to_vec(&file)
            .map_err(|e| AinariError::internal(format!("Failed to serialize '{path}': {e}")))?,
    );
    encrypt_bytes(&plain, keys.metadata_key(), path.as_bytes())
}

/// Decrypts a metadata-file.
///
/// # Arguments
///
/// * `path` - Path of the file relative to the directory of the backup
/// * `content` - The encrypted file
/// * `keys` - Keys of the backup
///
/// # Returns
///
/// The content of the file, or `InvalidInput`, if it was changed, belongs to another path or has
/// an unknown version.
pub fn decrypt_metadata<T: DeserializeOwned>(
    path: &str,
    content: &[u8],
    keys: &BackupKeys,
) -> Result<T, AinariError> {
    let plain = zeroize::Zeroizing::new(
        decrypt_bytes(content, keys.metadata_key(), path.as_bytes()).map_err(|_| {
            AinariError::InvalidInput(format!("File '{path}' of the backup can't be decrypted."))
        })?,
    );

    let file: MetadataFile<serde_json::Value> = serde_json::from_slice(&plain)
        .map_err(|e| AinariError::InvalidInput(format!("File '{path}' is invalid: {e}")))?;
    if file.schema_version == 0 || file.schema_version > METADATA_SCHEMA_VERSION {
        return Err(AinariError::InvalidInput(format!(
            "File '{path}' has the unknown schema-version {}.",
            file.schema_version
        )));
    }

    serde_json::from_value(file.data)
        .map_err(|e| AinariError::InvalidInput(format!("File '{path}' is invalid: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TEST_KDF_PARAMS;
    use ainari_common::secret::Secret;

    fn keys(passphrase: &str) -> BackupKeys {
        BackupKeys::derive(
            &Secret::from(passphrase),
            b"0123456789abcdef",
            &TEST_KDF_PARAMS,
        )
        .unwrap()
    }

    fn metadata() -> RyokanMetadata {
        RyokanMetadata {
            images: vec![ImageExport {
                uuid: Uuid::new_v4(),
                name: "image".to_string(),
                file_type: "qcow2".to_string(),
                file_size: 42,
                is_snapshot: false,
                secret_uuid: Uuid::new_v4(),
            }],
            files_included: true,
        }
    }

    #[test]
    fn test_metadata_round_trip() {
        let keys = keys("passphrase");
        let data = metadata();
        let encrypted = encrypt_metadata(RYOKAN_METADATA, &data, &keys).unwrap();
        // the content is not readable without the key
        assert!(!String::from_utf8_lossy(&encrypted).contains("qcow2"));

        let decrypted: RyokanMetadata =
            decrypt_metadata(RYOKAN_METADATA, &encrypted, &keys).unwrap();
        assert_eq!(decrypted, data);
    }

    #[test]
    fn test_metadata_is_bound_to_key_and_path() {
        let keys = keys("passphrase");
        let encrypted = encrypt_metadata(RYOKAN_METADATA, &metadata(), &keys).unwrap();

        let result: Result<RyokanMetadata, _> =
            decrypt_metadata(RYOKAN_METADATA, &encrypted, &self::keys("other"));
        assert!(matches!(result, Err(AinariError::InvalidInput(_))));

        // a file, which was moved to another path
        let result: Result<RyokanMetadata, _> =
            decrypt_metadata(SAKURA_METADATA, &encrypted, &keys);
        assert!(matches!(result, Err(AinariError::InvalidInput(_))));

        // a manipulated file
        let mut manipulated = encrypted.clone();
        let last = manipulated.len() - 1;
        manipulated[last] ^= 1;
        let result: Result<RyokanMetadata, _> =
            decrypt_metadata(RYOKAN_METADATA, &manipulated, &keys);
        assert!(matches!(result, Err(AinariError::InvalidInput(_))));
    }

    #[test]
    fn test_unknown_schema_version() {
        let keys = keys("passphrase");
        let file = MetadataFile {
            schema_version: METADATA_SCHEMA_VERSION + 1,
            data: metadata(),
        };
        let plain = serde_json::to_vec(&file).unwrap();
        let encrypted =
            encrypt_bytes(&plain, keys.metadata_key(), RYOKAN_METADATA.as_bytes()).unwrap();
        let result: Result<RyokanMetadata, _> =
            decrypt_metadata(RYOKAN_METADATA, &encrypted, &keys);
        assert!(matches!(result, Err(AinariError::InvalidInput(msg)) if msg.contains("schema")));
    }

    #[test]
    fn test_paths() {
        let uuid = Uuid::new_v4();
        assert_eq!(disk_path(&uuid), format!("disks/{uuid}.enc"));
        assert_eq!(image_path(&uuid), format!("images/{uuid}.enc"));
    }
}
