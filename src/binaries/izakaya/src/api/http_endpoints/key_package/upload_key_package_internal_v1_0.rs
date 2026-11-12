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

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openmls::prelude::{tls_codec::*, *};
use openmls_rust_crypto::RustCrypto;
use validator::Validate;

use crate::database::key_package_table;

use ainari_api::error_mapping::map_db_write_error;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "key_package",
    summary = "Upload key-packages",
    description = r###"Upload MLS key-packages of a gateway, so other gateways can invite it into their groups.

Every key-package is validated before it is stored: it has to be correctly signed, must not be
expired and the identity of its credential has to be the `client_id` of the request. If one of
them is invalid, none of them is stored."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 500
)]
pub async fn upload_key_package_internal(
    body: Json<KeyPackageUploadReq>,
    context: UserContext,
) -> Result<CreatedJson<KeyPackageUploadResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    for (index, key_package) in body.key_packages.iter().enumerate() {
        validate_key_package(&body.client_id, key_package).map_err(|e| {
            ErrorResponse::BadRequest(format!("Invalid key-package at position {index}: {e}"))
        })?;
    }

    let uuids = key_package_table::add_key_packages(&body.client_id, &body.key_packages, &context)
        .map_err(|e| {
            map_db_write_error(
                &format!("add key-packages of client '{}'", body.client_id),
                e,
                Some(&context),
            )
        })?;

    log::debug!(
        "Stored {} key-package(s) of client '{}'",
        uuids.len(),
        body.client_id
    );

    Ok(CreatedJson(KeyPackageUploadResp {
        client_id: body.client_id.clone(),
        uuids,
    }))
}

/// Checks, that an uploaded key-package is valid and belongs to a MLS-client.
///
/// # Arguments
/// * `client_id` - Identity of the client, which uploaded the key-package
/// * `encoded_key_package` - Base64-encoded TLS-serialized key-package
///
/// # Returns
/// `Ok(())` if the key-package can be shared, otherwise the reason why not
fn validate_key_package(client_id: &str, encoded_key_package: &str) -> Result<(), String> {
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use openmls_rust_crypto::OpenMlsRustCrypto;

    const CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

    /// Creates a base64-encoded key-package of a new client with the given identity.
    pub(crate) fn new_key_package(identity: &str) -> String {
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
