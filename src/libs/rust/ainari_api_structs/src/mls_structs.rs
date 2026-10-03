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

//! Structs of the izakaya, which shares the MLS key-packages of the gateways and delivers the
//! MLS-messages between them.
//!
//! All MLS-objects travel as base64-encoded TLS-serialization, so izakaya only has to understand
//! them for the validation of a key-package and otherwise just stores and forwards them.
//!
//! Beside that, izakaya coordinates the changes of the groups (see `MlsOperation` and
//! `MlsRoundMessage`). Who may be member of which group is not decided by izakaya, but by hanami,
//! which signs a `MlsGrant` for every gateway of a network. The gateways check these grants
//! themselves, so neither a compromised izakaya nor a compromised gateway can bring a gateway into
//! a group, which hanami didn't grant.

use std::fmt;
use std::str::FromStr;

use apistos::ApiComponent;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, Verifier};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

pub use ed25519_dalek::{SigningKey, VerifyingKey};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct KeyPackageUploadReq {
    /// Identity of the MLS-client, which owns the key-packages. It has to match the identity in
    /// the credential of each key-package.
    #[validate(length(min = 1, max = 256))]
    pub client_id: String,
    /// Base64-encoded TLS-serialized key-packages, at most 100 with one request
    #[validate(length(min = 1, max = 100))]
    pub key_packages: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct KeyPackageUploadResp {
    pub client_id: String,
    pub uuids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct KeyPackageResp {
    pub uuid: Uuid,
    pub client_id: String,
    /// Base64-encoded TLS-serialized key-package
    pub key_package: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct KeyPackageCountResp {
    pub client_id: String,
    pub count: u64,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, JsonSchema, ApiComponent, Validate)]
pub struct KeyPackageClaimReq {
    /// Base64-encoded MLS signature-key, which the key-package has to have. The committer passes
    /// the key, which hanami granted, so a key-package, which someone else uploaded in the name of
    /// the gateway, is never handed out.
    #[serde(default)]
    pub signature_key: Option<String>,
}

/// Addresses the key-packages of one MLS-client
#[derive(Debug, Deserialize, JsonSchema, ApiComponent)]
pub struct ClientPath {
    pub client_id: String,
}

/// Type of a MLS-message, which is delivered by the izakaya
#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, ApiComponent,
)]
#[serde(rename_all = "lowercase")]
pub enum MlsMessageType {
    /// Invitation of a new member into a group
    Welcome,
    /// Change of the group, which moves all existing members into the next epoch
    Commit,
    /// Change of the group, which the committer of the group has to make, as JSON-encoded
    /// `MlsOperation`. Only izakaya sends it.
    Operation,
    /// Step of the key-rotation of a group, as JSON-encoded `MlsRoundMessage`. Only izakaya sends
    /// it.
    Round,
}

impl fmt::Display for MlsMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MlsMessageType::Welcome => "welcome",
            MlsMessageType::Commit => "commit",
            MlsMessageType::Operation => "operation",
            MlsMessageType::Round => "round",
        };
        write!(f, "{s}")
    }
}

impl FromStr for MlsMessageType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "welcome" => Ok(MlsMessageType::Welcome),
            "commit" => Ok(MlsMessageType::Commit),
            "operation" => Ok(MlsMessageType::Operation),
            "round" => Ok(MlsMessageType::Round),
            other => Err(format!(
                "Unknown message-type '{other}', expected welcome, commit, operation or round"
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MlsMessageReq {
    /// Group, which the message belongs to
    #[validate(length(min = 1, max = 256))]
    pub group_id: String,
    /// Epoch of the group, which the message leads into. The messages of a group are delivered
    /// in the order of their epochs.
    pub epoch: u64,
    pub message_type: MlsMessageType,
    /// Identity of the MLS-client, which sent the message
    #[validate(length(min = 1, max = 256))]
    pub sender: String,
    /// Identities of the MLS-clients, which have to receive the message
    #[validate(length(min = 1))]
    pub recipients: Vec<String>,
    /// Base64-encoded TLS-serialized MLS-message
    #[validate(length(min = 1))]
    pub payload: String,
    /// Grants of all gateways, which a commit adds to the group, so every member can check, that
    /// hanami allowed them
    #[serde(default)]
    pub grants: Vec<MlsGrant>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct MlsMessageCreateResp {
    pub uuids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct MlsMessageResp {
    pub uuid: Uuid,
    pub group_id: String,
    pub epoch: u64,
    pub message_type: MlsMessageType,
    pub sender: String,
    pub recipient: String,
    /// Base64-encoded TLS-serialized MLS-message, or the JSON-encoded operation or round-step
    pub payload: String,
    /// Grants of all gateways, which a commit adds to the group
    #[serde(default)]
    pub grants: Vec<MlsGrant>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, JsonSchema, ApiComponent)]
pub struct MlsMessageListResp {
    pub messages: Vec<MlsMessageResp>,
}

/// Addresses one pending message of a MLS-client
#[derive(Debug, Deserialize, JsonSchema, ApiComponent)]
pub struct MlsMessagePath {
    pub client_id: String,
    pub message_uuid: Uuid,
}

// ================================================================================================
// membership-grants
// ================================================================================================

/// Decision of hanami about the membership of a gateway in the group of a network
#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, ApiComponent,
)]
#[serde(rename_all = "lowercase")]
pub enum MlsGrantAction {
    /// The gateway may be member of the group
    Add,
    /// The gateway must not be member of the group anymore
    Remove,
}

/// Content of a membership-grant, which hanami signs
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, ApiComponent)]
pub struct MlsGrantPayload {
    /// Tenant of the network, whose group is meant
    pub vni: u32,
    /// Identity of the gateway, which is its underlay-address
    pub client_id: String,
    /// Base64-encoded public MLS signature-key of the gateway. A key-package or a member with
    /// another key is not covered by the grant, even if it claims the same identity.
    pub signature_key: String,
    pub action: MlsGrantAction,
    /// Unix-time in seconds, when hanami signed the grant
    pub issued_at: i64,
    /// Unix-time in seconds, after which a gateway is not added with the grant anymore
    pub expires_at: i64,
}

impl MlsGrantPayload {
    /// Checks, if the grant is expired.
    ///
    /// # Arguments
    /// * `now` - Current unix-time in seconds
    ///
    /// # Returns
    /// `true` if the grant must not be used to add a gateway anymore
    pub fn is_expired_at(&self, now: i64) -> bool {
        now > self.expires_at
    }
}

/// Membership-grant, signed by hanami.
///
/// The signature covers the exact bytes of the payload, which travel base64-encoded, so the
/// grant can't be changed on its way without breaking the signature.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, ApiComponent)]
pub struct MlsGrant {
    /// Base64-encoded JSON of the `MlsGrantPayload`
    pub payload: String,
    /// Base64-encoded Ed25519-signature of the decoded payload
    pub signature: String,
}

impl MlsGrant {
    /// Signs a grant.
    ///
    /// # Arguments
    /// * `payload` - Content of the grant
    /// * `key` - Signing-key of hanami
    ///
    /// # Returns
    /// The signed grant
    pub fn sign(payload: &MlsGrantPayload, key: &SigningKey) -> MlsGrant {
        let bytes = serde_json::to_vec(payload).expect("a grant can always be serialized");
        let signature = key.sign(&bytes);
        MlsGrant {
            payload: BASE64.encode(&bytes),
            signature: BASE64.encode(signature.to_bytes()),
        }
    }

    /// Checks the signature of a grant and returns its content.
    ///
    /// The expiration is not checked here, because only the one, who adds a gateway with the
    /// grant, has to care about it (see `MlsGrantPayload::is_expired_at`).
    ///
    /// # Arguments
    /// * `key` - Public key of hanami
    ///
    /// # Returns
    /// The content of the grant, or the reason, why it is not valid
    pub fn verify(&self, key: &VerifyingKey) -> Result<MlsGrantPayload, String> {
        let bytes = BASE64
            .decode(&self.payload)
            .map_err(|e| format!("payload of the grant is not base64-encoded: {e}"))?;
        let signature = BASE64
            .decode(&self.signature)
            .map_err(|e| format!("signature of the grant is not base64-encoded: {e}"))?;
        let signature = Signature::from_slice(&signature)
            .map_err(|e| format!("signature of the grant is broken: {e}"))?;
        key.verify(&bytes, &signature)
            .map_err(|_| "signature of the grant is not valid".to_string())?;

        serde_json::from_slice(&bytes).map_err(|e| format!("payload of the grant is broken: {e}"))
    }
}

/// Reads the signing-key of hanami from its base64-encoded 32 byte seed.
///
/// # Arguments
/// * `encoded` - Base64-encoded seed
///
/// # Returns
/// The signing-key, or the reason, why it can't be read
pub fn parse_signing_key(encoded: &str) -> Result<SigningKey, String> {
    let bytes = BASE64
        .decode(encoded.trim())
        .map_err(|e| format!("signing-key is not base64-encoded: {e}"))?;
    let seed: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "signing-key has to be a 32 byte seed".to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

/// Reads the public key of hanami.
///
/// # Arguments
/// * `encoded` - Base64-encoded 32 byte public key
///
/// # Returns
/// The public key, or the reason, why it can't be read
pub fn parse_verifying_key(encoded: &str) -> Result<VerifyingKey, String> {
    let bytes = BASE64
        .decode(encoded.trim())
        .map_err(|e| format!("public key is not base64-encoded: {e}"))?;
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "public key has to be 32 bytes long".to_string())?;
    VerifyingKey::from_bytes(&bytes).map_err(|e| format!("public key is not valid: {e}"))
}

/// Encodes the public key, which belongs to a signing-key, for the configs of izakaya and torii.
///
/// # Arguments
/// * `key` - Signing-key of hanami
///
/// # Returns
/// The base64-encoded public key
pub fn encode_verifying_key(key: &SigningKey) -> String {
    BASE64.encode(key.verifying_key().to_bytes())
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MlsGrantReq {
    pub grant: MlsGrant,
}

// ================================================================================================
// coordination of the groups
// ================================================================================================

/// Addresses the group of a network
#[derive(Debug, Deserialize, JsonSchema, ApiComponent)]
pub struct MlsGroupVniPath {
    pub vni: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MlsSubscribeReq {
    /// Identity of the gateway
    #[validate(length(min = 1, max = 256))]
    pub client_id: String,
    /// Whether the gateway still holds the group of the network, for example after a restart
    pub has_group: bool,
}

/// What a gateway has to do after it subscribed to the group of a network
#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, ApiComponent,
)]
#[serde(rename_all = "lowercase")]
pub enum MlsSubscribeAction {
    /// There is no group yet, the gateway creates it and becomes its committer
    Create,
    /// The gateway is member of the group already
    Member,
    /// The gateway is added by the committer, it gets a welcome
    Joining,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct MlsSubscribeResp {
    pub action: MlsSubscribeAction,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MlsUnsubscribeReq {
    /// Identity of the gateway
    #[validate(length(min = 1, max = 256))]
    pub client_id: String,
}

/// Kind of a change of a group, which the committer makes
#[derive(
    Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, JsonSchema, ApiComponent,
)]
#[serde(rename_all = "lowercase")]
pub enum MlsOperationKind {
    /// Add a gateway, or add it again, if it lost the group
    Add,
    /// Remove a gateway
    Remove,
    /// Rotate the keys with a self-update of the committer
    Update,
}

impl fmt::Display for MlsOperationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MlsOperationKind::Add => "add",
            MlsOperationKind::Remove => "remove",
            MlsOperationKind::Update => "update",
        };
        write!(f, "{s}")
    }
}

impl FromStr for MlsOperationKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "add" => Ok(MlsOperationKind::Add),
            "remove" => Ok(MlsOperationKind::Remove),
            "update" => Ok(MlsOperationKind::Update),
            other => Err(format!("Unknown operation '{other}'")),
        }
    }
}

/// Change of a group, which izakaya hands to the committer of the group
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct MlsOperation {
    pub op_uuid: Uuid,
    pub vni: u32,
    pub kind: MlsOperationKind,
    /// Gateway, which is added or removed
    #[serde(default)]
    pub client_id: Option<String>,
    /// Grant of hanami for the gateway, which is added
    #[serde(default)]
    pub grant: Option<MlsGrant>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MlsOperationDoneReq {
    pub op_uuid: Uuid,
    /// Identity of the committer
    #[validate(length(min = 1, max = 256))]
    pub client_id: String,
    /// Whether the change was made. A refused change, for example because of an invalid grant,
    /// is dropped.
    pub success: bool,
    /// Epoch of the group after the change
    pub epoch: u64,
    /// Identities of all members after the change
    pub members: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Step of a key-rotation
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    JsonSchema,
    ApiComponent,
)]
#[serde(rename_all = "lowercase")]
pub enum MlsRoundPhase {
    /// Install the incoming keys of the new epoch next to the old ones
    Install,
    /// Switch the outgoing traffic to the keys of the new epoch
    Switch,
    /// Remove the incoming keys of the old epochs
    Cleanup,
}

impl fmt::Display for MlsRoundPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MlsRoundPhase::Install => "install",
            MlsRoundPhase::Switch => "switch",
            MlsRoundPhase::Cleanup => "cleanup",
        };
        write!(f, "{s}")
    }
}

impl FromStr for MlsRoundPhase {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "install" => Ok(MlsRoundPhase::Install),
            "switch" => Ok(MlsRoundPhase::Switch),
            "cleanup" => Ok(MlsRoundPhase::Cleanup),
            other => Err(format!("Unknown round-phase '{other}'")),
        }
    }
}

/// Step of a key-rotation, which izakaya hands to every member of a group
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct MlsRoundMessage {
    pub vni: u32,
    pub epoch: u64,
    pub phase: MlsRoundPhase,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MlsRoundAckReq {
    /// Identity of the gateway
    #[validate(length(min = 1, max = 256))]
    pub client_id: String,
    pub epoch: u64,
    pub phase: MlsRoundPhase,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_type_travels_as_its_lowercase_name() {
        assert_eq!(
            serde_json::to_string(&MlsMessageType::Welcome).unwrap(),
            "\"welcome\""
        );
        assert_eq!(MlsMessageType::Commit.to_string(), "commit");
        assert_eq!(
            "commit".parse::<MlsMessageType>().unwrap(),
            MlsMessageType::Commit
        );
        assert!("Commit".parse::<MlsMessageType>().is_err());
        assert_eq!(
            "round".parse::<MlsMessageType>().unwrap(),
            MlsMessageType::Round
        );
    }

    fn payload() -> MlsGrantPayload {
        MlsGrantPayload {
            vni: 5,
            client_id: "10.0.0.5".to_string(),
            signature_key: "a2V5".to_string(),
            action: MlsGrantAction::Add,
            issued_at: 100,
            expires_at: 200,
        }
    }

    #[test]
    fn a_signed_grant_is_verified_with_the_public_key() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let grant = MlsGrant::sign(&payload(), &key);

        let public_key = parse_verifying_key(&encode_verifying_key(&key)).unwrap();
        assert_eq!(grant.verify(&public_key).unwrap(), payload());

        // another key didn't sign it
        let other = SigningKey::from_bytes(&[8u8; 32]);
        assert!(grant.verify(&other.verifying_key()).is_err());
    }

    #[test]
    fn a_changed_grant_is_refused() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut grant = MlsGrant::sign(&payload(), &key);

        let mut changed = payload();
        changed.client_id = "10.0.0.6".to_string();
        grant.payload = BASE64.encode(serde_json::to_vec(&changed).unwrap());
        assert!(grant.verify(&key.verifying_key()).is_err());
    }

    #[test]
    fn a_grant_expires() {
        assert!(!payload().is_expired_at(200));
        assert!(payload().is_expired_at(201));
    }

    #[test]
    fn keys_are_read_from_their_base64_form() {
        let seed = BASE64.encode([7u8; 32]);
        let key = parse_signing_key(&seed).unwrap();
        assert_eq!(key.to_bytes(), [7u8; 32]);
        assert!(parse_signing_key(&BASE64.encode([7u8; 16])).is_err());
        assert!(parse_verifying_key("not base64!").is_err());
    }
}
