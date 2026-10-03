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

//! Validation of the MLS key-packages, which the gateways upload.
//!
//! The izakaya doesn't take part in any group, but it checks every key-package before it is
//! shared with other gateways: it has to be well-formed, correctly signed, not expired and has to
//! belong to the client, which uploaded it. A gateway, which claims a key-package later, still
//! validates it on its own, but a broken or foreign key-package is already refused here.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openmls::prelude::{tls_codec::*, *};
use openmls_rust_crypto::RustCrypto;

/// Checks, that an uploaded key-package is valid and belongs to a MLS-client.
///
/// # Arguments
/// * `client_id` - Identity of the client, which uploaded the key-package
/// * `encoded_key_package` - Base64-encoded TLS-serialized key-package
///
/// # Returns
/// `Ok(())` if the key-package can be shared, otherwise the reason why not
pub fn validate_key_package(client_id: &str, encoded_key_package: &str) -> Result<(), String> {
    let bytes = BASE64
        .decode(encoded_key_package)
        .map_err(|e| format!("key-package is not base64-encoded: {e}"))?;

    let key_package_in = KeyPackageIn::tls_deserialize_exact(bytes.as_slice())
        .map_err(|e| format!("key-package can not be decoded: {e}"))?;

    let key_package = key_package_in
        .validate(&RustCrypto::default(), ProtocolVersion::Mls10)
        .map_err(|e| format!("key-package is invalid: {e}"))?;

    let credential = BasicCredential::try_from(key_package.leaf_node().credential().clone())
        .map_err(|e| format!("key-package has no basic credential: {e}"))?;

    if credential.identity() != client_id.as_bytes() {
        return Err(format!(
            "key-package belongs to '{}' instead of '{client_id}'",
            String::from_utf8_lossy(credential.identity())
        ));
    }

    Ok(())
}

/// Reads the MLS signature-key of a key-package.
///
/// # Arguments
/// * `encoded_key_package` - Base64-encoded TLS-serialized key-package
///
/// # Returns
/// The base64-encoded signature-key, or `None` if the key-package can't be read
pub fn signature_key_of(encoded_key_package: &str) -> Option<String> {
    let bytes = BASE64.decode(encoded_key_package).ok()?;
    let key_package = KeyPackageIn::tls_deserialize_exact(bytes.as_slice())
        .ok()?
        .validate(&RustCrypto::default(), ProtocolVersion::Mls10)
        .ok()?;
    Some(BASE64.encode(key_package.leaf_node().signature_key().as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openmls_rust_crypto::OpenMlsRustCrypto;

    const CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

    /// Creates a base64-encoded key-package of a new client with the given identity.
    fn new_key_package(identity: &str) -> String {
        let provider = OpenMlsRustCrypto::default();
        let signer =
            openmls_basic_credential::SignatureKeyPair::new(CIPHERSUITE.signature_algorithm())
                .unwrap();
        let credential = CredentialWithKey {
            credential: BasicCredential::new(identity.as_bytes().to_vec()).into(),
            signature_key: signer.to_public_vec().into(),
        };
        let bundle = KeyPackage::builder()
            .build(CIPHERSUITE, &provider, &signer, credential)
            .unwrap();
        BASE64.encode(bundle.key_package().tls_serialize_detached().unwrap())
    }

    #[test]
    fn a_key_package_of_the_uploading_client_is_accepted() {
        let key_package = new_key_package("10.0.0.5");
        assert!(validate_key_package("10.0.0.5", &key_package).is_ok());
    }

    #[test]
    fn a_key_package_of_another_client_is_refused() {
        let key_package = new_key_package("10.0.0.6");
        let err = validate_key_package("10.0.0.5", &key_package).unwrap_err();
        assert!(err.contains("10.0.0.6"));
    }

    #[test]
    fn the_signature_key_of_a_key_package_is_read() {
        let key_package = new_key_package("10.0.0.5");
        let key = signature_key_of(&key_package).unwrap();
        assert_eq!(BASE64.decode(key).unwrap().len(), 32);
        assert!(signature_key_of("garbage").is_none());
    }

    #[test]
    fn garbage_is_refused() {
        assert!(validate_key_package("10.0.0.5", "not base64!").is_err());
        assert!(validate_key_package("10.0.0.5", &BASE64.encode(b"garbage")).is_err());
    }

    #[test]
    fn a_tampered_key_package_is_refused() {
        let mut bytes = BASE64.decode(new_key_package("10.0.0.5")).unwrap();
        // flip a bit within the signed content, which breaks the signature
        let middle = bytes.len() / 2;
        bytes[middle] ^= 0x01;
        assert!(validate_key_package("10.0.0.5", &BASE64.encode(bytes)).is_err());
    }
}
