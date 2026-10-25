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

//! Encryption of in-memory data with AES-256-GCM.
//!
//! The encrypted data has the layout `nonce || ciphertext || tag`, which is the same like the
//! encrypted files of `ainari_files`, so with empty additional data it can also be decrypted by
//! `ainari_files::file_encryption::decrypt_file`.

use crate::error::AinariError;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngExt;

/// Length of the encryption key in bytes (256 bit)
pub const ENCRYPTION_KEY_LEN: usize = 32;
/// Length of the nonce, which is recommended for GCM
const NONCE_LEN: usize = 12;
/// Length of the authentication tag of GCM
const TAG_LEN: usize = 16;

/// Encrypts in-memory data using AES-256-GCM encryption
///
/// # Arguments
///
/// * `plaintext` - The data to encrypt
/// * `key` - The raw encryption key
/// * `aad` - Additional data, which is not encrypted, but authenticated, so the ciphertext can be
///   bound to its context (for example the name of the file). The same data must be given for
///   decryption. Can be empty.
///
/// # Returns
///
/// * `Result<Vec<u8>, AinariError>` - The nonce, the ciphertext and the tag, or an error
///
/// # Errors
///
/// Returns `AinariError::InternalError` for encryption-related errors
pub fn encrypt_bytes(
    plaintext: &[u8],
    key: &[u8; ENCRYPTION_KEY_LEN],
    aad: &[u8],
) -> Result<Vec<u8>, AinariError> {
    // Generate a random nonce for each call, because GCM must never re-use a nonce with a key
    let mut nonce = [0u8; NONCE_LEN];
    rand::rng().fill(&mut nonce);

    // the result of aes-gcm already contains the tag at the end of the ciphertext
    let ciphertext = Aes256Gcm::new(key.into())
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| AinariError::internal("Error while encrypting data"))?;

    let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypts in-memory data encrypted with `encrypt_bytes`
///
/// # Arguments
///
/// * `data` - The nonce, the ciphertext and the tag
/// * `key` - The raw decryption key
/// * `aad` - The same additional data, which was given for encryption
///
/// # Returns
///
/// * `Result<Vec<u8>, AinariError>` - The decrypted data or an error
///
/// # Errors
///
/// Returns `AinariError::InvalidInput` if:
/// * The data is too short
/// * The authentication fails (indicating tampering, wrong key or wrong additional data)
pub fn decrypt_bytes(
    data: &[u8],
    key: &[u8; ENCRYPTION_KEY_LEN],
    aad: &[u8],
) -> Result<Vec<u8>, AinariError> {
    if data.len() < NONCE_LEN + TAG_LEN {
        return Err(AinariError::InvalidInput(
            "Input data too short".to_string(),
        ));
    }
    let (nonce, ciphertext) = data.split_at(NONCE_LEN);
    let nonce = Nonce::try_from(nonce)
        .map_err(|_| AinariError::InvalidInput("Invalid nonce length".to_string()))?;

    Aes256Gcm::new(key.into())
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| AinariError::InvalidInput("Invalid key or tampered data".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8; ENCRYPTION_KEY_LEN] = b"0123456789abcdef0123456789abcdef";
    const OTHER_KEY: &[u8; ENCRYPTION_KEY_LEN] = b"0123456789abcdef0123456789abcdeg";

    #[test]
    fn encrypt_decrypt_bytes_roundtrip() {
        for plaintext in [&b""[..], b"x", b"{\"schema_version\": 1, \"data\": {}}"] {
            let encrypted = encrypt_bytes(plaintext, KEY, b"aad").unwrap();
            assert_eq!(encrypted.len(), NONCE_LEN + plaintext.len() + TAG_LEN);
            if !plaintext.is_empty() {
                assert!(
                    !encrypted
                        .windows(plaintext.len())
                        .any(|window| window == plaintext),
                    "plaintext is visible in the encrypted data"
                );
            }
            assert_eq!(decrypt_bytes(&encrypted, KEY, b"aad").unwrap(), plaintext);
        }
    }

    #[test]
    fn encrypt_bytes_uses_random_nonce() {
        let first = encrypt_bytes(b"same data", KEY, b"").unwrap();
        let second = encrypt_bytes(b"same data", KEY, b"").unwrap();
        assert_ne!(first[..NONCE_LEN], second[..NONCE_LEN]);
        assert_ne!(first, second);
    }

    #[test]
    fn decrypt_bytes_rejects_manipulated_data() {
        let encrypted = encrypt_bytes(b"some metadata", KEY, b"aad").unwrap();

        // flipping a single bit anywhere (nonce, ciphertext or tag) must be detected
        for pos in 0..encrypted.len() {
            let mut manipulated = encrypted.clone();
            manipulated[pos] ^= 0x01;
            let result = decrypt_bytes(&manipulated, KEY, b"aad");
            assert!(
                matches!(result, Err(AinariError::InvalidInput(_))),
                "manipulation at byte {pos} not detected"
            );
        }

        // truncated, extended and too short data
        let truncated = &encrypted[..encrypted.len() - 1];
        assert!(decrypt_bytes(truncated, KEY, b"aad").is_err());
        let mut extended = encrypted.clone();
        extended.push(0);
        assert!(decrypt_bytes(&extended, KEY, b"aad").is_err());
        let too_short = &encrypted[..NONCE_LEN + TAG_LEN - 1];
        assert!(matches!(
            decrypt_bytes(too_short, KEY, b"aad"),
            Err(AinariError::InvalidInput(_))
        ));
    }

    #[test]
    fn decrypt_bytes_rejects_wrong_key_or_aad() {
        let encrypted = encrypt_bytes(b"some metadata", KEY, b"miko.json.enc").unwrap();
        assert!(matches!(
            decrypt_bytes(&encrypted, OTHER_KEY, b"miko.json.enc"),
            Err(AinariError::InvalidInput(_))
        ));
        assert!(matches!(
            decrypt_bytes(&encrypted, KEY, b"hanami.json.enc"),
            Err(AinariError::InvalidInput(_))
        ));
        assert!(matches!(
            decrypt_bytes(&encrypted, KEY, b""),
            Err(AinariError::InvalidInput(_))
        ));
    }
}
