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

//! The MLS-groups of the gateway: their creation and removal, their members and the
//! key-packages, with which other gateways invite this one.
//!
//! Nothing in here talks to the izakaya or to the database.

use crate::core::mls_key_exchange::identity::{CIPHERSUITE, MlsIdentity, group_id_of, identity_of};
use crate::core::mls_key_exchange::state::MlsState;
use ainari_api::errors::ErrorResponse;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openmls::prelude::{tls_codec::*, *};

/// Number of past epochs of a group, whose messages can still be decrypted, so a message, which
/// was encrypted shortly before a commit, can still be read after it
pub const MAX_PAST_EPOCHS: usize = 16;

/// Logs an error of openmls and turns it into the generic `InternalError`.
pub fn internal_error(action: &str, e: impl std::fmt::Debug) -> ErrorResponse {
    ainari_api::report_internal_error!(None, "Failed to {action}: {e:?}");
    ErrorResponse::InternalError("Internal Error".to_string())
}

/// Returns the identity of the gateway, which `MlsState::ensure_identity` has to create before.
pub fn own_identity(identity: &Option<MlsIdentity>) -> Result<&MlsIdentity, ErrorResponse> {
    identity.as_ref().ok_or_else(|| {
        ainari_api::report_internal_error!(
            None,
            "The MLS-identity of the gateway was not created yet"
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })
}

/// Serializes an outgoing MLS-message for the izakaya.
pub fn encode(message: &MlsMessageOut) -> Result<String, ErrorResponse> {
    message
        .tls_serialize_detached()
        .map(|bytes| BASE64.encode(bytes))
        .map_err(|e| internal_error("serialize a MLS-message", e))
}

/// Lists the identities of all members of a group, which have to follow a change of it.
///
/// # Arguments
/// * `group` - The group
/// * `own_client_id` - Identity of this gateway, which made the change itself
/// * `excluded` - Identities of the members, which are removed or re-added with the change
///
/// # Returns
/// The identities of the other members
pub fn other_members(group: &MlsGroup, own_client_id: &str, excluded: &[&str]) -> Vec<String> {
    let mut members: Vec<String> = group
        .members()
        .filter_map(|member| identity_of(&member.credential))
        .filter(|id| id != own_client_id && !excluded.contains(&id.as_str()))
        .collect();
    members.sort();
    members.dedup();
    members
}

/// Creates new key-packages of the gateway, which another gateway uses to invite it.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `count` - Number of key-packages
///
/// # Returns
/// The base64-encoded key-packages
pub fn create_key_packages(state: &MlsState, count: u32) -> Result<Vec<String>, ErrorResponse> {
    let identity = own_identity(&state.identity)?;

    (0..count)
        .map(|_| {
            let bundle = KeyPackage::builder()
                .build(
                    CIPHERSUITE,
                    &state.provider,
                    &identity.signer,
                    identity.credential.clone(),
                )
                .map_err(|e| internal_error("create a key-package", e))?;
            bundle
                .key_package()
                .tls_serialize_detached()
                .map(|bytes| BASE64.encode(bytes))
                .map_err(|e| internal_error("serialize a key-package", e))
        })
        .collect()
}

/// Configuration of a new group. The ratchet-tree is part of every welcome, so an invited
/// gateway needs nothing else than the welcome to join.
fn create_config() -> MlsGroupCreateConfig {
    MlsGroupCreateConfig::builder()
        .ciphersuite(CIPHERSUITE)
        .use_ratchet_tree_extension(true)
        .max_past_epochs(MAX_PAST_EPOCHS)
        .build()
}

/// Configuration of a group, which is joined by a welcome
pub(crate) fn join_config() -> MlsGroupJoinConfig {
    MlsGroupJoinConfig::builder()
        .use_ratchet_tree_extension(true)
        .max_past_epochs(MAX_PAST_EPOCHS)
        .build()
}

/// Creates the group of a network with this gateway as its only member.
///
/// An already existing group of the network is replaced, because the control plane only creates
/// a group, if no other gateway serves the network anymore.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `vni` - Tenant of the network
///
/// # Returns
/// `true` if an older group of the network was replaced, otherwise `false`
pub fn create_group(state: &mut MlsState, vni: u32) -> Result<bool, ErrorResponse> {
    let replaced = match state.groups.remove(&vni) {
        Some(mut old_group) => {
            old_group
                .delete(state.provider.storage())
                .map_err(|e| internal_error("delete the old MLS-group", e))?;
            true
        }
        None => false,
    };

    let identity = own_identity(&state.identity)?;
    let group = MlsGroup::new_with_group_id(
        &state.provider,
        &identity.signer,
        &create_config(),
        group_id_of(vni),
        identity.credential.clone(),
    )
    .map_err(|e| internal_error("create the MLS-group", e))?;

    state.groups.insert(vni, group);
    Ok(replaced)
}

/// Removes the group of a network from the gateway.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// `Ok(())`, or `NotFound` if the gateway is not a member of the group
pub fn delete_group(state: &mut MlsState, vni: u32) -> Result<(), ErrorResponse> {
    let mut group = state
        .groups
        .remove(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
    group
        .delete(state.provider.storage())
        .map_err(|e| internal_error("delete the MLS-group", e))
}

/// Decodes and validates a key-package, which was claimed from the izakaya.
///
/// The izakaya validates the key-packages already, but it is not trusted with the decision, who
/// is invited: the identity in the credential has to be the gateway, which should be invited.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `client_id` - Identity of the gateway, which should be invited
/// * `encoded` - Base64-encoded TLS-serialized key-package
///
/// # Returns
/// The validated key-package, or `BadRequest` if it is not usable
pub fn decode_key_package(
    state: &MlsState,
    client_id: &str,
    encoded: &str,
) -> Result<KeyPackage, ErrorResponse> {
    let invalid = |reason: String| {
        ErrorResponse::BadRequest(format!("Invalid key-package of '{client_id}': {reason}"))
    };

    let bytes = BASE64.decode(encoded).map_err(|e| invalid(e.to_string()))?;
    let key_package = KeyPackageIn::tls_deserialize_exact(bytes.as_slice())
        .map_err(|e| invalid(e.to_string()))?
        .validate(state.provider.crypto(), ProtocolVersion::Mls10)
        .map_err(|e| invalid(e.to_string()))?;

    match identity_of(key_package.leaf_node().credential()) {
        Some(identity) if identity == client_id => Ok(key_package),
        Some(identity) => Err(invalid(format!("it belongs to '{identity}'"))),
        None => Err(invalid("it has no basic credential".to_string())),
    }
}

/// Checks, if a gateway is a member of the group of a network.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
/// * `client_id` - Identity of the gateway
///
/// # Returns
/// `true` if the gateway is a member
pub fn is_member(state: &MlsState, vni: u32, client_id: &str) -> bool {
    state.groups.get(&vni).is_some_and(|group| {
        group
            .members()
            .any(|member| identity_of(&member.credential).as_deref() == Some(client_id))
    })
}

/// Lists the identities of all members of the group of a network.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// The identities, ordered, or none, if the gateway is not a member
pub fn members(state: &MlsState, vni: u32) -> Vec<String> {
    let mut members: Vec<String> = state
        .groups
        .get(&vni)
        .map(|group| {
            group
                .members()
                .filter_map(|member| identity_of(&member.credential))
                .collect()
        })
        .unwrap_or_default();
    members.sort();
    members
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::test_support::{invite, new_gateway, secret};

    #[test]
    fn invited_gateways_share_the_secret_of_the_group() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");

        assert!(!create_group(&mut a, 5).unwrap());
        invite(&mut a, &mut b, &mut [], 5);
        assert_eq!(secret(&a, 5), secret(&b, 5));

        // any member can invite the next gateway, the others follow with the commit
        invite(&mut b, &mut c, &mut [&mut a], 5);
        assert_eq!(secret(&a, 5), secret(&c, 5));
        assert_eq!(secret(&b, 5), secret(&c, 5));
        assert_eq!(a.groups[&5].epoch().as_u64(), 2);
        assert_eq!(members(&c, 5), vec!["10.0.0.1", "10.0.0.2", "10.0.0.3"]);
    }

    #[test]
    fn a_key_package_of_another_gateway_is_refused() {
        let a = new_gateway("10.0.0.1");
        let b = new_gateway("10.0.0.2");
        let encoded = create_key_packages(&b, 1).unwrap().remove(0);

        assert!(decode_key_package(&a, "10.0.0.2", &encoded).is_ok());
        assert!(matches!(
            decode_key_package(&a, "10.0.0.3", &encoded),
            Err(ErrorResponse::BadRequest(_))
        ));
    }

    #[test]
    fn a_gateway_can_be_invited_again_after_it_lost_its_group() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        // b forgot the group, for example because its last VM of the network was deleted
        delete_group(&mut b, 5).unwrap();
        assert!(is_member(&a, 5, "10.0.0.2"));

        // the old leaf goes with the commit, which adds the new one
        invite(&mut a, &mut b, &mut [], 5);

        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_eq!(members(&a, 5).len(), 2);
        assert_eq!(a.groups[&5].members().count(), 2);
    }
}
