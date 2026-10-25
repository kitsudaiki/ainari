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

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use ainari_api_structs::backup_structs::BackupKind;
use ainari_common::error::AinariError;
use ainari_common::kdf::{KdfParams, generate_kdf_salt};
use ainari_common::secret::Secret;

use super::keys::BackupKeys;

/// Name of the manifest within the directory of a backup
pub const MANIFEST_FILE: &str = "manifest.json";

/// Version of the format of the backups, which are written by this version of isekai. Backups
/// with a newer format are rejected, because their content can't be interpreted correctly.
pub const FORMAT_VERSION: u32 = 1;

type HmacSha256 = Hmac<Sha256>;

/// File of a backup, like it is listed in its manifest.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile {
    /// path of the file relative to the directory of the backup
    pub path: String,
    /// size of the file in bytes
    pub size: u64,
    /// hex-encoded sha256 of the content of the file
    pub sha256: String,
}

/// Parameters of the derivation of the keys from the passphrase of a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ManifestKdf {
    pub params: KdfParams,
    /// base64-encoded random salt
    pub salt: String,
}

impl ManifestKdf {
    /// Creates the parameters for a new backup with a new random salt.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters of Argon2id
    ///
    /// # Returns
    ///
    /// The parameters of the key-derivation.
    pub fn new(params: KdfParams) -> Self {
        ManifestKdf {
            params,
            salt: BASE64.encode(generate_kdf_salt()),
        }
    }

    /// Derives the keys of the backup from its passphrase. This is expensive on purpose.
    ///
    /// # Arguments
    ///
    /// * `passphrase` - Passphrase of the backup
    ///
    /// # Returns
    ///
    /// The keys, or `InvalidInput`, if the parameters or the salt are invalid.
    pub fn derive_keys(&self, passphrase: &Secret) -> Result<BackupKeys, AinariError> {
        let salt = BASE64.decode(&self.salt).map_err(|_| {
            AinariError::InvalidInput("Salt of the manifest is invalid".to_string())
        })?;
        BackupKeys::derive(passphrase, &salt, &self.params)
    }
}

/// Manifest of a backup.
///
/// It describes the backup and lists all of its files with their size and checksum. It is the
/// last file, which is written into a backup, so a backup without a valid manifest is
/// incomplete. The manifest itself is stored as plain json, so the parameters of the
/// key-derivation can be read before the keys are known, but it is authenticated with a HMAC,
/// which is checked before any of its content is trusted. A wrong passphrase gives another key,
/// so it is detected by the HMAC as well.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Manifest {
    pub format_version: u32,
    /// version of ainari, which has created the backup
    pub ainari_version: String,
    pub kind: BackupKind,
    pub backup_uuid: Uuid,
    /// project of the backup at the time of the backup
    pub project_id: String,
    /// saved virtual_machine of a backup of the kind `VIRTUAL_MACHINE`
    pub virtual_machine_uuid: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub kdf: ManifestKdf,
    pub files: Vec<ManifestFile>,
}

/// Content of the manifest-file: the manifest and its HMAC.
///
/// The manifest is kept as generic json-value, so the HMAC is calculated over exactly the same
/// serialization, when it is written and when it is checked.
#[derive(Serialize, Deserialize)]
struct SignedManifest {
    manifest: serde_json::Value,
    /// hex-encoded HMAC-SHA256 of the serialized manifest
    hmac: String,
}

/// Part of the manifest, which can be read before the manifest is authenticated.
#[derive(Deserialize)]
struct ManifestHeader {
    format_version: u32,
    kdf: ManifestKdf,
}

impl Manifest {
    /// Serializes the manifest together with its HMAC into the content of the manifest-file.
    ///
    /// # Arguments
    ///
    /// * `keys` - Keys of the backup
    ///
    /// # Returns
    ///
    /// The content of the manifest-file.
    pub fn seal(&self, keys: &BackupKeys) -> Result<Vec<u8>, AinariError> {
        let manifest = serde_json::to_value(self)
            .map_err(|e| AinariError::internal(format!("Failed to serialize manifest: {e}")))?;
        let hmac = hex::encode(calculate_hmac(&manifest, keys)?);
        serde_json::to_vec_pretty(&SignedManifest { manifest, hmac })
            .map_err(|e| AinariError::internal(format!("Failed to serialize manifest: {e}")))
    }

    /// Gets a file of the manifest.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file relative to the directory of the backup
    ///
    /// # Returns
    ///
    /// The file, or `None`, if it is not part of the backup.
    pub fn file(&self, path: &str) -> Option<&ManifestFile> {
        self.files.iter().find(|file| file.path == path)
    }

    /// Checks size and checksum of a file of the backup against the manifest.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file relative to the directory of the backup
    /// * `size` - Actual size of the file
    /// * `sha256` - Actual hex-encoded sha256 of the file
    ///
    /// # Returns
    ///
    /// Ok, if the file is unchanged, or `InvalidInput`, if it is missing in the manifest or was
    /// changed.
    pub fn check_file(&self, path: &str, size: u64, sha256: &str) -> Result<(), AinariError> {
        let expected = self.file(path).ok_or_else(|| {
            AinariError::InvalidInput(format!("File '{path}' is not part of the backup."))
        })?;
        if expected.size != size || !expected.sha256.eq_ignore_ascii_case(sha256) {
            return Err(AinariError::InvalidInput(format!(
                "File '{path}' of the backup was changed."
            )));
        }
        Ok(())
    }

    /// Checks the content of a file of the backup against the manifest.
    ///
    /// # Arguments
    ///
    /// * `path` - Path of the file relative to the directory of the backup
    /// * `content` - Content of the file
    ///
    /// # Returns
    ///
    /// Ok, if the file is unchanged, or `InvalidInput`, if it is missing in the manifest or was
    /// changed.
    pub fn check_content(&self, path: &str, content: &[u8]) -> Result<(), AinariError> {
        self.check_file(path, content.len() as u64, &sha256_hex(content))
    }
}

/// Manifest, which was read from a backup, but is not authenticated yet.
pub struct UnverifiedManifest {
    manifest: serde_json::Value,
    hmac: Vec<u8>,
    kdf: ManifestKdf,
}

impl UnverifiedManifest {
    /// Parses the content of a manifest-file.
    ///
    /// Only the format-version and the parameters of the key-derivation are read, which are
    /// required to derive the keys, which authenticate the rest of the manifest.
    ///
    /// # Arguments
    ///
    /// * `content` - Content of the manifest-file
    ///
    /// # Returns
    ///
    /// The unverified manifest, or `InvalidInput`, if the file is no manifest or has an unknown
    /// format-version.
    pub fn parse(content: &[u8]) -> Result<Self, AinariError> {
        let invalid =
            |reason: &str| AinariError::InvalidInput(format!("Invalid manifest: {reason}"));

        let signed: SignedManifest =
            serde_json::from_slice(content).map_err(|e| invalid(&e.to_string()))?;
        let header: ManifestHeader =
            serde_json::from_value(signed.manifest.clone()).map_err(|e| invalid(&e.to_string()))?;
        if header.format_version == 0 || header.format_version > FORMAT_VERSION {
            return Err(AinariError::InvalidInput(format!(
                "Backup has the unknown format-version {}. This version of isekai supports the \
                 format-versions up to {FORMAT_VERSION}.",
                header.format_version
            )));
        }
        let hmac = hex::decode(&signed.hmac).map_err(|_| invalid("hmac is not hex-encoded"))?;

        Ok(UnverifiedManifest {
            manifest: signed.manifest,
            hmac,
            kdf: header.kdf,
        })
    }

    /// Parameters of the key-derivation of the backup.
    pub fn kdf(&self) -> &ManifestKdf {
        &self.kdf
    }

    /// Authenticates the manifest with the keys of the backup.
    ///
    /// # Arguments
    ///
    /// * `keys` - Keys, which were derived from the passphrase
    ///
    /// # Returns
    ///
    /// The authenticated manifest, or `InvalidInput`, if the passphrase is wrong or the manifest
    /// was changed.
    pub fn verify(self, keys: &BackupKeys) -> Result<Manifest, AinariError> {
        let mut mac = new_mac(keys)?;
        mac.update(&serialize(&self.manifest)?);
        mac.verify_slice(&self.hmac).map_err(|_| {
            AinariError::InvalidInput(
                "Wrong passphrase, or the manifest of the backup was changed.".to_string(),
            )
        })?;

        serde_json::from_value(self.manifest)
            .map_err(|e| AinariError::InvalidInput(format!("Invalid manifest: {e}")))
    }
}

/// Calculates the hex-encoded sha256 of a content.
///
/// # Arguments
///
/// * `content` - The content
///
/// # Returns
///
/// The hex-encoded checksum.
pub fn sha256_hex(content: &[u8]) -> String {
    hex::encode(Sha256::digest(content))
}

fn new_mac(keys: &BackupKeys) -> Result<HmacSha256, AinariError> {
    <HmacSha256 as KeyInit>::new_from_slice(keys.manifest_mac_key())
        .map_err(|e| AinariError::internal(format!("Failed to initialize hmac: {e}")))
}

fn serialize(manifest: &serde_json::Value) -> Result<Vec<u8>, AinariError> {
    serde_json::to_vec(manifest)
        .map_err(|e| AinariError::internal(format!("Failed to serialize manifest: {e}")))
}

fn calculate_hmac(manifest: &serde_json::Value, keys: &BackupKeys) -> Result<Vec<u8>, AinariError> {
    let mut mac = new_mac(keys)?;
    mac.update(&serialize(manifest)?);
    Ok(mac.finalize().into_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TEST_KDF_PARAMS;

    const PASSPHRASE: &str = "correct horse battery";

    fn new_manifest(kdf: ManifestKdf) -> Manifest {
        Manifest {
            format_version: FORMAT_VERSION,
            ainari_version: "0.22.0".to_string(),
            kind: BackupKind::VirtualMachine,
            backup_uuid: Uuid::new_v4(),
            project_id: "test-project".to_string(),
            virtual_machine_uuid: Some(Uuid::new_v4()),
            created_at: Utc::now(),
            kdf,
            files: vec![
                ManifestFile {
                    path: "metadata/sakura.json.enc".to_string(),
                    size: 5,
                    sha256: sha256_hex(b"hello"),
                },
                ManifestFile {
                    path: "disks/vm.enc".to_string(),
                    size: 1 << 40,
                    sha256: "ab".repeat(32),
                },
            ],
        }
    }

    /// Creates a sealed manifest together with the keys, which were used for it.
    fn sealed() -> (Manifest, Vec<u8>, BackupKeys) {
        let kdf = ManifestKdf::new(TEST_KDF_PARAMS);
        let keys = kdf.derive_keys(&Secret::from(PASSPHRASE)).unwrap();
        let manifest = new_manifest(kdf);
        let content = manifest.seal(&keys).unwrap();
        (manifest, content, keys)
    }

    #[test]
    fn test_round_trip() {
        let (manifest, content, _) = sealed();

        // the keys are derived again from the parameters within the manifest
        let unverified = UnverifiedManifest::parse(&content).unwrap();
        let keys = unverified
            .kdf()
            .derive_keys(&Secret::from(PASSPHRASE))
            .unwrap();
        assert_eq!(unverified.verify(&keys).unwrap(), manifest);
    }

    #[test]
    fn test_wrong_passphrase() {
        let (_, content, _) = sealed();
        let unverified = UnverifiedManifest::parse(&content).unwrap();
        let keys = unverified
            .kdf()
            .derive_keys(&Secret::from("wrong passphrase"))
            .unwrap();
        assert!(matches!(
            unverified.verify(&keys),
            Err(AinariError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_manipulated_manifest() {
        let (_, content, keys) = sealed();

        // a changed checksum of a file within the manifest
        let mut signed: SignedManifest = serde_json::from_slice(&content).unwrap();
        signed.manifest["files"][1]["sha256"] = serde_json::json!("cd".repeat(32));
        let manipulated = serde_json::to_vec(&signed).unwrap();
        let unverified = UnverifiedManifest::parse(&manipulated).unwrap();
        assert!(matches!(
            unverified.verify(&keys),
            Err(AinariError::InvalidInput(_))
        ));

        // a changed hmac
        let mut signed: SignedManifest = serde_json::from_slice(&content).unwrap();
        signed.hmac = "00".repeat(32);
        let manipulated = serde_json::to_vec(&signed).unwrap();
        let unverified = UnverifiedManifest::parse(&manipulated).unwrap();
        assert!(unverified.verify(&keys).is_err());

        // no manifest at all
        assert!(UnverifiedManifest::parse(b"{}").is_err());
        assert!(UnverifiedManifest::parse(b"no json").is_err());
    }

    #[test]
    fn test_unknown_format_version() {
        let kdf = ManifestKdf::new(TEST_KDF_PARAMS);
        let keys = kdf.derive_keys(&Secret::from(PASSPHRASE)).unwrap();
        for version in [0, FORMAT_VERSION + 1] {
            let mut manifest = new_manifest(kdf.clone());
            manifest.format_version = version;
            let content = manifest.seal(&keys).unwrap();
            // rejected before the keys are derived
            assert!(matches!(
                UnverifiedManifest::parse(&content),
                Err(AinariError::InvalidInput(msg)) if msg.contains("format-version")
            ));
        }
    }

    #[test]
    fn test_check_files() {
        let (manifest, _, _) = sealed();

        assert!(
            manifest
                .check_content("metadata/sakura.json.enc", b"hello")
                .is_ok()
        );
        // the checksum is compared independent of its case
        assert!(
            manifest
                .check_file("disks/vm.enc", 1 << 40, &"AB".repeat(32))
                .is_ok()
        );

        // manipulated file
        assert!(matches!(
            manifest.check_content("metadata/sakura.json.enc", b"hellO"),
            Err(AinariError::InvalidInput(_))
        ));
        // other size
        assert!(
            manifest
                .check_file("disks/vm.enc", 1, &"ab".repeat(32))
                .is_err()
        );
        // file, which is not part of the backup
        assert!(matches!(
            manifest.check_content("metadata/other.json.enc", b"hello"),
            Err(AinariError::InvalidInput(_))
        ));
    }

    #[test]
    fn test_sha256_hex() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
