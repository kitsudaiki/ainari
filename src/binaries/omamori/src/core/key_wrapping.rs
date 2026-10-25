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
use base64::engine::general_purpose::STANDARD;
use uuid::Uuid;
use zeroize::Zeroizing;

use ainari_common::encryption::{ENCRYPTION_KEY_LEN, decrypt_bytes, encrypt_bytes};
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;

/// Key, which wraps the payloads of the secrets, while they are outside of the omamori, for
/// example within a backup. The key is provided by the caller for each request and is never
/// stored. Its bytes are wiped, when it is dropped.
pub struct WrapKey(Zeroizing<[u8; ENCRYPTION_KEY_LEN]>);

impl WrapKey {
    /// Reads the key from its base64-encoding.
    ///
    /// # Arguments
    ///
    /// * `key_b64` - The base64-encoded key with 256 bit
    ///
    /// # Returns
    ///
    /// * `Ok(WrapKey)` - The key
    /// * `Err(AinariError::InvalidInput)` - The key is not base64-encoded or has the wrong length
    pub fn from_base64(key_b64: &Secret) -> Result<Self, AinariError> {
        let decoded = Zeroizing::new(STANDARD.decode(key_b64.reveal()).map_err(|_| {
            AinariError::InvalidInput("The wrap-key is not base64-encoded.".to_string())
        })?);
        let mut key = Zeroizing::new([0u8; ENCRYPTION_KEY_LEN]);
        if decoded.len() != ENCRYPTION_KEY_LEN {
            return Err(AinariError::InvalidInput(format!(
                "The wrap-key must have {ENCRYPTION_KEY_LEN} bytes."
            )));
        }
        key.copy_from_slice(&decoded);
        Ok(WrapKey(key))
    }
}

/// Wraps the payload of a secret with the wrap-key (AES-256-GCM with a new nonce).
///
/// The UUID of the secret is used as additional data, so a wrapped payload can't be assigned to
/// another secret without being detected.
///
/// # Arguments
///
/// * `payload` - The plain payload of the secret
/// * `key` - The wrap-key
/// * `secret_uuid` - The UUID of the secret
///
/// # Returns
///
/// * `Ok(String)` - The base64-encoded wrapped payload
/// * `Err(AinariError)` - The encryption failed
pub fn wrap_payload(
    payload: &Secret,
    key: &WrapKey,
    secret_uuid: &Uuid,
) -> Result<String, AinariError> {
    let wrapped = encrypt_bytes(
        payload.reveal().as_bytes(),
        &key.0,
        secret_uuid.to_string().as_bytes(),
    )?;
    Ok(STANDARD.encode(wrapped))
}

/// Unwraps the payload of a secret, which was wrapped with `wrap_payload`.
///
/// # Arguments
///
/// * `wrapped_b64` - The base64-encoded wrapped payload
/// * `key` - The wrap-key
/// * `secret_uuid` - The UUID of the secret, which the payload was wrapped for
///
/// # Returns
///
/// * `Ok(Secret)` - The plain payload
/// * `Err(AinariError::InvalidInput)` - The payload is invalid, manipulated, belongs to another
///   secret or the key is wrong
pub fn unwrap_payload(
    wrapped_b64: &str,
    key: &WrapKey,
    secret_uuid: &Uuid,
) -> Result<Secret, AinariError> {
    let invalid = || {
        AinariError::InvalidInput(format!(
            "The payload of secret '{secret_uuid}' can't be unwrapped. Either the wrap-key is \
             wrong or the payload was changed."
        ))
    };
    let wrapped = STANDARD.decode(wrapped_b64).map_err(|_| invalid())?;
    let plain = Zeroizing::new(
        decrypt_bytes(&wrapped, &key.0, secret_uuid.to_string().as_bytes())
            .map_err(|_| invalid())?,
    );
    let plain = String::from_utf8(plain.to_vec()).map_err(|_| invalid())?;
    Ok(Secret::from(plain))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> WrapKey {
        WrapKey::from_base64(&Secret::from(STANDARD.encode([byte; ENCRYPTION_KEY_LEN]))).unwrap()
    }

    #[test]
    fn test_wrap_round_trip() {
        let secret_uuid = Uuid::new_v4();
        let payload = Secret::from("my secret payload".to_string());

        let wrapped = wrap_payload(&payload, &key(1), &secret_uuid).unwrap();
        assert!(!wrapped.contains("my secret payload"));
        // a new nonce for every wrapping
        assert_ne!(
            wrapped,
            wrap_payload(&payload, &key(1), &secret_uuid).unwrap()
        );

        let unwrapped = unwrap_payload(&wrapped, &key(1), &secret_uuid).unwrap();
        assert_eq!(unwrapped.reveal(), payload.reveal());
    }

    #[test]
    fn test_unwrap_errors() {
        let secret_uuid = Uuid::new_v4();
        let payload = Secret::from("payload".to_string());
        let wrapped = wrap_payload(&payload, &key(1), &secret_uuid).unwrap();

        // wrong key, other secret, manipulated or invalid payload
        assert!(unwrap_payload(&wrapped, &key(2), &secret_uuid).is_err());
        assert!(unwrap_payload(&wrapped, &key(1), &Uuid::new_v4()).is_err());
        let mut manipulated = STANDARD.decode(&wrapped).unwrap();
        let last = manipulated.len() - 1;
        manipulated[last] ^= 1;
        let manipulated = STANDARD.encode(manipulated);
        assert!(unwrap_payload(&manipulated, &key(1), &secret_uuid).is_err());
        assert!(unwrap_payload("no base64!", &key(1), &secret_uuid).is_err());
    }

    #[test]
    fn test_invalid_wrap_keys() {
        assert!(WrapKey::from_base64(&Secret::from("no base64!".to_string())).is_err());
        let short = Secret::from(STANDARD.encode([1u8; 16]));
        assert!(WrapKey::from_base64(&short).is_err());
    }
}
