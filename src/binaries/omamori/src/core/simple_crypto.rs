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

use aes_gcm::aead::{Aead, KeyInit, generic_array::GenericArray};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use rand::RngCore; // needed to use .encode() and .decode()
use uuid::Uuid;

use crate::config;
use crate::database::simple_crypto_table;

use super::crypto_trait::*;

use ainari_common::enums;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

const NONCE_SIZE: usize = 12; // 96 bits
const KEY_SIZE: usize = 32; // 256 bits

/// A simple cryptographic module that provides basic encryption and decryption functionality
/// using AES-256-GCM algorithm.
pub struct SimpleCrypto {
    #[allow(dead_code)]
    pub name: String,
}

impl SimpleCrypto {
    /// Creates a new instance of SimpleCrypto.
    ///
    /// # Returns
    /// A new SimpleCrypto instance with the name "simple_crypto".
    pub fn new() -> Self {
        SimpleCrypto {
            name: "simple_crypto".to_owned(),
        }
    }

    /// Encrypts the given plaintext using AES-256-GCM algorithm.
    ///
    /// # Arguments
    /// * `plaintext` - The secret to be encrypted
    /// * `key_b64` - The encryption key in base64 format
    ///
    /// # Returns
    /// Result containing the base64 encoded ciphertext (nonce + ciphertext concatenated)
    /// or an AinariError if encryption fails.
    ///
    /// # Errors
    /// * AinariError::InvalidInput - If the key is invalid or encryption fails
    fn encrypt(&self, plaintext: &Secret, key_b64: &Secret) -> Result<String, AinariError> {
        let key_bytes = decode_base64_key(key_b64)?;

        let key = GenericArray::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.reveal().as_bytes())
            .map_err(|_| AinariError::InvalidInput("Failed to encrypt plaintext".to_string()))?;

        // output: nonce + ciphertext, base64 encoded
        let mut combined = Vec::new();
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(STANDARD.encode(&combined))
    }

    /// Decrypts the given ciphertext using AES-256-GCM algorithm.
    ///
    /// # Arguments
    /// * `encrypted_secret_b64` - The base64 encoded ciphertext to decrypt
    /// * `key_b64` - The decryption key in base64 format
    ///
    /// # Returns
    /// Result containing the decrypted secret or an AinariError if decryption fails.
    ///
    /// # Errors
    /// * AinariError::InvalidInput - If the input is invalid or decryption fails
    fn decrypt(&self, encrypted_secret_b64: &str, key_b64: &Secret) -> Result<Secret, AinariError> {
        let key_bytes = decode_base64_key(key_b64)?;
        let key = GenericArray::from_slice(&key_bytes);

        let encrypted_secret_bytes =
            STANDARD
                .decode(encrypted_secret_b64.as_bytes())
                .map_err(|_| {
                    AinariError::InvalidInput(
                        "Provided encrypted-secret is not a valid base64-encoded string."
                            .to_string(),
                    )
                })?;

        if encrypted_secret_bytes.len() < NONCE_SIZE {
            let msg = "Provided encrpted secret is too short.".to_string();
            return Err(AinariError::InvalidInput(msg));
        }

        let (nonce_bytes, ciphertext) = encrypted_secret_bytes.split_at(NONCE_SIZE);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext_bytes = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| AinariError::InvalidInput("Failed to decrypt secret".to_string()))?;

        let plaintext = String::from_utf8(plaintext_bytes).map_err(|_| {
            AinariError::InvalidInput("Failed to convert decrypted secret into text.".to_string())
        })?;

        Ok(Secret::from(plaintext))
    }
}

/// Decodes a base64 encoded key and validates its length.
///
/// # Arguments
/// * `key_b64` - The base64 encoded key
///
/// # Returns
/// Result containing the decoded key bytes or an AinariError if decoding fails
/// or the key length is invalid.
///
/// # Errors
/// * AinariError::InvalidInput - If the key is invalid or has wrong length
fn decode_base64_key(key_b64: &Secret) -> Result<Vec<u8>, AinariError> {
    // decode base64 key
    let key_bytes = STANDARD.decode(key_b64.reveal().as_bytes()).map_err(|_| {
        AinariError::InvalidInput("Provided key is not a valid base64-encoded string.".to_string())
    })?;

    if key_bytes.len() != KEY_SIZE {
        let msg = format!(
            "Invalid key length: expected {}, got {}",
            KEY_SIZE,
            key_bytes.len()
        );
        return Err(AinariError::InvalidInput(msg));
    }

    Ok(key_bytes)
}

impl CryptoModule for SimpleCrypto {
    /// Stores a secret in the database after encrypting it.
    ///
    /// # Arguments
    /// * `secret_uuid` - The UUID of the secret to store
    /// * `plaintext` - The secret to store
    ///
    /// # Returns
    /// Result indicating success or failure of the operation.
    ///
    /// # Errors
    /// * AinariError::InvalidInput - If encryption fails or secret UUID is not found
    /// * AinariError::InternalError - If database operation fails
    fn store(&self, secret_uuid: &Uuid, plaintext: &Secret) -> Result<(), AinariError> {
        let key_b64 = &config::CONFIG.simple_crypto.key_b64;
        let encrypted_secret = self.encrypt(plaintext, key_b64)?;

        // add new secret to datbase
        simple_crypto_table::add_new_simple_crypto_data(secret_uuid, &encrypted_secret).map_err(
            |_| {
                AinariError::InternalError(format!(
                    "Failed to add simple-crypto-secret with UUID '{secret_uuid}' to database."
                ))
            },
        )?;

        Ok(())
    }

    /// Retrieves a secret from the database and decrypts it.
    ///
    /// # Arguments
    /// * `secret_uuid` - The UUID of the secret to retrieve
    ///
    /// # Returns
    /// Result containing the decrypted secret or an AinariError if retrieval fails.
    ///
    /// # Errors
    /// * AinariError::InvalidInput - If decryption fails or secret UUID is not found
    /// * AinariError::InternalError - If database operation fails
    fn retrieve(&self, secret_uuid: &Uuid) -> Result<Secret, AinariError> {
        let secret_data = match simple_crypto_table::get_secret(secret_uuid) {
            Ok(secret_data) => secret_data,
            Err(enums::DbError::InternalError) => {
                return Err(AinariError::InternalError("".to_string()));
            }
            Err(enums::DbError::NotFound) => {
                let msg = format!("Secret with UUID '{secret_uuid}' not found.");
                return Err(AinariError::InvalidInput(msg));
            }
        };

        let key_b64 = &config::CONFIG.simple_crypto.key_b64;
        self.decrypt(&secret_data.encrypted_secret, key_b64)
    }

    /// Deletes a secret from the database.
    ///
    /// # Arguments
    /// * `secret_uuid` - The UUID of the secret to delete
    ///
    /// # Returns
    /// Result indicating success or failure of the operation.
    ///
    /// # Errors
    /// * AinariError::InvalidInput - If secret UUID is not found
    /// * AinariError::InternalError - If database operation fails
    fn delete(&self, secret_uuid: &Uuid) -> Result<(), AinariError> {
        // delete secret from database
        match simple_crypto_table::delete_secret(secret_uuid) {
            Ok(_) => Ok(()),
            Err(enums::DbError::InternalError) => Err(AinariError::InternalError("".to_string())),
            Err(enums::DbError::NotFound) => {
                let msg = format!("Secret with UUID '{secret_uuid}' not found.");
                Err(AinariError::InvalidInput(msg))
            }
        }
    }

    /// Gets the name of the cryptographic module.
    ///
    /// # Returns
    /// The name of the module as a String.
    fn get_name(&self) -> String {
        self.name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_ok() {
        let simple_crypto = SimpleCrypto::new();
        let key_b64 = Secret::from("q9vN4CjOQm5wKzyzjZtS7t4oQp8oQK1JvU5xgq8vFzE=");
        let plaintext = Secret::from("this is a test-text for encryption");

        assert_eq!(simple_crypto.get_name(), "simple_crypto".to_string());

        let ret_enc = simple_crypto.encrypt(&plaintext, &key_b64);
        assert!(ret_enc.is_ok());

        let encrypted_secret_b64 = ret_enc.unwrap();
        println!("test-output: {}", encrypted_secret_b64);

        let ret_dec = simple_crypto.decrypt(&encrypted_secret_b64, &key_b64);
        assert!(ret_dec.is_ok());

        let decrypted_plaintext = ret_dec.unwrap();
        assert_eq!(decrypted_plaintext.reveal(), plaintext.reveal());
    }

    #[test]
    fn test_encrypt_decrypt_err() {
        let simple_crypto = SimpleCrypto::new();
        let key_b64 = Secret::from("q9vN4CjOQm5wKzyzjZtS7t4oQp8oQK1JvU5xgq8vFzE=");
        let other_key_b64 = Secret::from("Wv7gD9jR5mM8zX4oU1kTb2eYvG0qHp9wF3sLrNdChKI=");
        let invalid_key_b64 = Secret::from("oQK1JvU5xgq8vFzE=");
        let plaintext = Secret::from("this is a test-text for encryption");

        // test with invalid key
        let ret_enc = simple_crypto.encrypt(&plaintext, &invalid_key_b64);
        assert!(ret_enc.is_err());

        let encrypted_secret_b64 = simple_crypto.encrypt(&plaintext, &key_b64).unwrap();

        // test decrypt with invalid key
        let ret_dec1 = simple_crypto.decrypt(&encrypted_secret_b64, &invalid_key_b64);
        assert!(ret_dec1.is_err());

        // test decrypt with other key
        let ret_dec2 = simple_crypto.decrypt(&encrypted_secret_b64, &other_key_b64);
        assert!(ret_dec2.is_err());
    }
}
