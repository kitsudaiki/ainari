// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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

use super::crypto_trait::*;

use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

const NONCE_SIZE: usize = 12; // 96 bits
const KEY_SIZE: usize = 32; // 256 bits

pub struct SimpleCrypto {
    #[allow(dead_code)]
    pub name: String,
}

impl SimpleCrypto {
    pub fn new() -> Self {
        SimpleCrypto {
            name: "simple_crypto".to_owned(),
        }
    }
}

fn decode_base64_key(key_b64: &Secret) -> Result<Vec<u8>, AinariError> {
    // decode base64 key
    let key_bytes = match STANDARD.decode(key_b64.reveal().as_bytes()) {
        Ok(key_bytes) => key_bytes,
        Err(_) => {
            // HINT (kitsudaiki): do NOT use the error-message of the decode-function to avoid the risk
            // of printing information of the key in the log-output
            let msg = "Provided key is not a valid base64-encoded string.".to_string();
            return Err(AinariError::InvalidInput(msg));
        }
    };
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
    fn encrypt(&self, plaintext: &Secret, key_b64: &Secret) -> Result<String, AinariError> {
        let key_bytes = decode_base64_key(key_b64)?;

        let key = GenericArray::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        // generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // encrypt
        let ciphertext = match cipher.encrypt(nonce, plaintext.reveal().as_bytes()) {
            Ok(ciphertext) => ciphertext,
            Err(_) => {
                let msg = "Failed to encrypt plaintext".to_string();
                return Err(AinariError::InvalidInput(msg));
            }
        };

        // output: nonce + ciphertext, base64 encoded
        let mut combined = Vec::new();
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(STANDARD.encode(&combined))
    }

    fn decrypt(&self, encrypted_secret_b64: &str, key_b64: &Secret) -> Result<Secret, AinariError> {
        let key_bytes = decode_base64_key(key_b64)?;
        let key = GenericArray::from_slice(&key_bytes);

        let encrypted_secret_bytes = match STANDARD.decode(encrypted_secret_b64.as_bytes()) {
            Ok(encrypted_secret_bytes) => encrypted_secret_bytes,
            Err(_) => {
                let msg =
                    "Provided encrypted-secret is not a valid base64-encoded string.".to_string();
                return Err(AinariError::InvalidInput(msg));
            }
        };

        if encrypted_secret_bytes.len() < NONCE_SIZE {
            let msg = "Provided encrpted secret is too short.".to_string();
            return Err(AinariError::InvalidInput(msg));
        }

        let (nonce_bytes, ciphertext) = encrypted_secret_bytes.split_at(NONCE_SIZE);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext_bytes = match cipher.decrypt(nonce, ciphertext) {
            Ok(plaintext_bytes) => plaintext_bytes,
            Err(_) => {
                let msg = "Failed to decrypt secret".to_string();
                return Err(AinariError::InvalidInput(msg));
            }
        };

        let plaintext = match String::from_utf8(plaintext_bytes) {
            Ok(plaintext) => plaintext,
            Err(_) => {
                let msg = "Failed to convert decrypted secret into text.".to_string();
                return Err(AinariError::InvalidInput(msg));
            }
        };
        Ok(Secret::from(plaintext))
    }

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
