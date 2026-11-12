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

//! Identity of the gateway within the MLS-groups and the ids of the groups.

use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use std::net::Ipv4Addr;

/// Ciphersuite of all groups. It is the one every MLS-implementation has to support.
pub const CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

/// Prefix of the id of a group, which is followed by the tenant of the network
const GROUP_ID_PREFIX: &str = "ainari-vni-";

/// Builds the id of the group of a network.
///
/// # Arguments
/// * `vni` - Tenant of the network
///
/// # Returns
/// The id of the group, like `ainari-vni-5`
pub fn group_id_of(vni: u32) -> GroupId {
    GroupId::from_slice(format!("{GROUP_ID_PREFIX}{vni}").as_bytes())
}

/// Reads the tenant of a network from the id of its group.
///
/// # Arguments
/// * `group_id` - Id of the group
///
/// # Returns
/// The tenant, or `None`, if the id was not built by `group_id_of`
pub fn vni_of(group_id: &GroupId) -> Option<u32> {
    std::str::from_utf8(group_id.as_slice())
        .ok()?
        .strip_prefix(GROUP_ID_PREFIX)?
        .parse()
        .ok()
}

/// Reads the identity of a member of a group, which is the client-id of its gateway.
///
/// # Arguments
/// * `credential` - Credential of the member
///
/// # Returns
/// The identity, or `None`, if the member has no basic credential
pub fn identity_of(credential: &Credential) -> Option<String> {
    BasicCredential::try_from(credential.clone())
        .ok()
        .map(|credential| String::from_utf8_lossy(credential.identity()).into_owned())
}

/// Identity of the gateway within all of its groups
pub struct MlsIdentity {
    /// Identity, which is written into the credential: the underlay-address of the gateway
    pub client_id: String,
    pub signer: SignatureKeyPair,
    pub credential: CredentialWithKey,
}

/// Builds the identity of the gateway from its signature-key-pair.
///
/// # Arguments
/// * `client_id` - Identity, which is written into the credential
/// * `signer` - Signature-key-pair of the gateway
///
/// # Returns
/// The identity
pub(crate) fn new_identity(client_id: String, signer: SignatureKeyPair) -> MlsIdentity {
    let credential = CredentialWithKey {
        credential: BasicCredential::new(client_id.as_bytes().to_vec()).into(),
        signature_key: signer.to_public_vec().into(),
    };
    MlsIdentity {
        client_id,
        signer,
        credential,
    }
}

/// Checks, if a restored MLS-identity belongs to another underlay-address than the current one.
///
/// # Arguments
/// * `client_id` - The restored identity
/// * `underlay_ip` - The current address of the underlay, if it has one
///
/// # Returns
/// `true` if the identity has to be replaced. Without an address of the underlay nothing can be
/// compared, so the identity is kept.
pub(crate) fn identity_outdated(client_id: &str, underlay_ip: Option<Ipv4Addr>) -> bool {
    underlay_ip.is_some_and(|ip| client_id != ip.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tenant_is_part_of_the_group_id() {
        assert_eq!(group_id_of(5).as_slice(), b"ainari-vni-5");
        assert_eq!(vni_of(&group_id_of(4711)), Some(4711));
        assert_eq!(vni_of(&GroupId::from_slice(b"foreign-group")), None);
        assert_eq!(vni_of(&GroupId::from_slice(b"ainari-vni-x")), None);
    }

    #[test]
    fn an_identity_of_another_address_is_replaced() {
        let ip = Ipv4Addr::new(10, 42, 8, 13);
        assert!(!identity_outdated("10.42.8.13", Some(ip)));
        assert!(identity_outdated("10.42.8.12", Some(ip)));
        // without an address there is nothing to compare
        assert!(!identity_outdated("10.42.8.12", None));
    }
}
