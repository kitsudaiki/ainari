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

use ainari_common::encryption::ENCRYPTION_KEY_LEN;
use ainari_common::error::AinariError;
use ainari_common::kdf::{DerivedKey, KdfParams, derive_master_key, expand_key};
use ainari_common::secret::Secret;

/// Label of the key, which encrypts the metadata-files of a backup
const METADATA_KEY_LABEL: &[u8] = b"ainari-backup-v1 metadata-key";
/// Label of the key, which wraps the payloads of the secrets within a backup
const WRAP_KEY_LABEL: &[u8] = b"ainari-backup-v1 wrap-key";
/// Label of the key, which authenticates the manifest of a backup
const MANIFEST_MAC_KEY_LABEL: &[u8] = b"ainari-backup-v1 manifest-mac-key";

/// Keys of a backup, which are derived from its passphrase.
///
/// The passphrase is turned into a master-key with Argon2id and the single keys are expanded from
/// it with HKDF, so every key has only one purpose. The keys are wiped from the memory, when they
/// are dropped.
pub struct BackupKeys {
    metadata_key: DerivedKey,
    wrap_key: DerivedKey,
    manifest_mac_key: DerivedKey,
}

impl BackupKeys {
    /// Derives the keys of a backup from its passphrase.
    ///
    /// This is expensive on purpose, so it should not run on a thread, which serves requests.
    ///
    /// # Arguments
    ///
    /// * `passphrase` - Passphrase of the backup
    /// * `salt` - Random salt, which is stored in the manifest of the backup
    /// * `params` - Parameters of Argon2id, which are stored in the manifest of the backup
    ///
    /// # Returns
    ///
    /// The keys, or `InvalidInput`, if the salt or the parameters are not valid.
    pub fn derive(
        passphrase: &Secret,
        salt: &[u8],
        params: &KdfParams,
    ) -> Result<Self, AinariError> {
        let master = derive_master_key(passphrase, salt, params)?;
        Ok(BackupKeys {
            metadata_key: expand_key(&master, METADATA_KEY_LABEL)?,
            wrap_key: expand_key(&master, WRAP_KEY_LABEL)?,
            manifest_mac_key: expand_key(&master, MANIFEST_MAC_KEY_LABEL)?,
        })
    }

    /// Key, which encrypts the metadata-files of the backup.
    pub fn metadata_key(&self) -> &[u8; ENCRYPTION_KEY_LEN] {
        self.metadata_key.as_bytes()
    }

    /// Key, which authenticates the manifest of the backup.
    pub fn manifest_mac_key(&self) -> &[u8] {
        self.manifest_mac_key.as_bytes()
    }

    /// Key, which wraps the payloads of the secrets, in the form, which omamori expects.
    ///
    /// # Returns
    ///
    /// The base64-encoded wrap-key.
    pub fn wrap_key_base64(&self) -> Secret {
        Secret::from(BASE64.encode(self.wrap_key.as_bytes()))
    }
}

impl std::fmt::Debug for BackupKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TEST_KDF_PARAMS as TEST_PARAMS;

    const SALT: &[u8] = b"0123456789abcdef";

    #[test]
    fn test_keys_are_different_and_reproducible() {
        let passphrase = Secret::from("correct horse battery");
        let keys = BackupKeys::derive(&passphrase, SALT, &TEST_PARAMS).unwrap();
        let again = BackupKeys::derive(&passphrase, SALT, &TEST_PARAMS).unwrap();

        // every key has its own purpose
        assert_ne!(keys.metadata_key(), keys.wrap_key.as_bytes());
        assert_ne!(&keys.metadata_key()[..], keys.manifest_mac_key());
        assert_ne!(keys.wrap_key.as_bytes()[..], *keys.manifest_mac_key());

        assert_eq!(keys.metadata_key(), again.metadata_key());
        assert_eq!(keys.manifest_mac_key(), again.manifest_mac_key());
        assert_eq!(keys.wrap_key_base64(), again.wrap_key_base64());
    }

    #[test]
    fn test_other_passphrase_or_salt_gives_other_keys() {
        let keys = BackupKeys::derive(&Secret::from("passphrase-1"), SALT, &TEST_PARAMS).unwrap();
        let other = BackupKeys::derive(&Secret::from("passphrase-2"), SALT, &TEST_PARAMS).unwrap();
        assert_ne!(keys.metadata_key(), other.metadata_key());

        let other_salt = BackupKeys::derive(
            &Secret::from("passphrase-1"),
            b"fedcba9876543210",
            &TEST_PARAMS,
        )
        .unwrap();
        assert_ne!(keys.manifest_mac_key(), other_salt.manifest_mac_key());
    }

    #[test]
    fn test_wrap_key_has_the_length_of_omamori() {
        let keys = BackupKeys::derive(&Secret::from("passphrase"), SALT, &TEST_PARAMS).unwrap();
        let decoded = BASE64.decode(keys.wrap_key_base64().reveal()).unwrap();
        assert_eq!(decoded.len(), ENCRYPTION_KEY_LEN);
        assert_eq!(format!("{keys:?}"), "***");
    }

    #[test]
    fn test_invalid_parameters_are_rejected() {
        let params = KdfParams {
            memory_kib: 1,
            ..TEST_PARAMS
        };
        assert!(matches!(
            BackupKeys::derive(&Secret::from("passphrase"), SALT, &params),
            Err(AinariError::InvalidInput(_))
        ));
        assert!(matches!(
            BackupKeys::derive(&Secret::from("passphrase"), b"short", &TEST_PARAMS),
            Err(AinariError::InvalidInput(_))
        ));
    }
}
