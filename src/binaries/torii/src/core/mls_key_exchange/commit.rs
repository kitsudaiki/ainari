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

//! Changes of the groups: the committer stages them, every member checks them.
//!
//! A change is only staged and hands back the messages, which the other gateways need to
//! follow it. The caller delivers them and merges the staged change only afterwards, so a
//! group never moves into an epoch, which the other members can't reach.

use crate::core::mls_key_exchange::group::{
    decode_key_package, encode, internal_error, other_members, own_identity,
};
use crate::core::mls_key_exchange::identity::identity_of;
use crate::core::mls_key_exchange::messages::OutgoingMessage;
use crate::core::mls_key_exchange::state::MlsState;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::{
    MlsGrant, MlsGrantAction, MlsGrantPayload, MlsMessageType, MlsOperationAdd,
    MlsOperationRefusal, VerifyingKey,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openmls::prelude::*;

/// Gateway, which a change adds to the group of a network
#[derive(Debug, Clone)]
pub struct Addition {
    /// Identity of the added gateway
    pub client_id: String,
    /// Validated key-package of the added gateway
    pub key_package: KeyPackage,
    /// Grant of hanami for the added gateway, which is attached to the commit, so the other
    /// members can check it
    pub grant: MlsGrant,
}

/// Checks the gateways, which a change should add to the group of a network, and claims their
/// key-packages.
///
/// A gateway is only added with a valid grant of hanami, which names exactly the identity and
/// the signature-key of its key-package. A gateway, which fails a check, is left out, the others
/// are added anyway.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
/// * `adds` - The gateways to add, with their grants
/// * `grant_key` - Public key of hanami
/// * `now` - Current unix-time in seconds
/// * `claim` - Claims a base64-encoded key-package of a gateway with the granted signature-key
///   (arguments: identity, signature-key)
///
/// # Returns
/// The checked additions and the gateways, which were left out
pub async fn check_additions<F, Fut>(
    state: &MlsState,
    vni: u32,
    adds: &[MlsOperationAdd],
    grant_key: &VerifyingKey,
    now: i64,
    mut claim: F,
) -> (Vec<Addition>, Vec<MlsOperationRefusal>)
where
    F: FnMut(String, String) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let mut additions = Vec::new();
    let mut refused = Vec::new();
    for add in adds {
        let target = add.client_id.as_str();
        let checked = async {
            let payload = verify_grant(&add.grant, grant_key, vni, target)?;
            if payload.is_expired_at(now) {
                return Err("the grant is expired".to_string());
            }
            let encoded = claim(target.to_string(), payload.signature_key.clone()).await?;
            let key_package =
                decode_key_package(state, target, &encoded).map_err(|e| format!("{e:?}"))?;
            let signature_key = BASE64.encode(key_package.leaf_node().signature_key().as_slice());
            if signature_key != payload.signature_key {
                return Err(format!(
                    "the key-package of '{target}' has another signature-key than granted"
                ));
            }
            Ok(key_package)
        }
        .await;

        match checked {
            Ok(key_package) => additions.push(Addition {
                client_id: target.to_string(),
                key_package,
                grant: add.grant.clone(),
            }),
            Err(reason) => refused.push(MlsOperationRefusal {
                client_id: target.to_string(),
                reason,
            }),
        }
    }
    (additions, refused)
}

/// Stages one change of the group of a network, which adds and removes any number of gateways
/// with a single commit.
///
/// A gateway, which is added, while it is still a member from an earlier membership (for
/// example, because it lost the group), loses its old leaf with the same commit.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `vni` - Tenant of the network
/// * `additions` - Gateways to add
/// * `removals` - Identities of the gateways to remove
///
/// # Returns
/// The commit for the remaining members, if there are any, followed by the welcome for the added
/// gateways, if there are any. `BadRequest` if the change is empty or removes this gateway
/// itself, `NotFound` if a removed gateway is not a member.
pub fn stage_change(
    state: &mut MlsState,
    vni: u32,
    additions: Vec<Addition>,
    removals: &[String],
) -> Result<Vec<OutgoingMessage>, ErrorResponse> {
    let identity = own_identity(&state.identity)?;
    let group = state
        .groups
        .get_mut(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
    validate_change(group, vni, &identity.client_id, &additions, removals)?;

    // the old leaves of the re-added gateways go together with the removed ones
    let mut excluded: Vec<&str> = removals.iter().map(String::as_str).collect();
    excluded.extend(additions.iter().map(|it| it.client_id.as_str()));
    let leaves = leaves_of(group, &excluded);

    let recipients = other_members(group, &identity.client_id, &excluded);
    let invited: Vec<String> = additions.iter().map(|it| it.client_id.clone()).collect();
    let grants: Vec<MlsGrant> = additions.iter().map(|it| it.grant.clone()).collect();

    let bundle = group
        .commit_builder()
        .propose_removals(leaves)
        .propose_adds(additions.into_iter().map(|it| it.key_package))
        .force_self_update(true)
        .load_psks(state.provider.storage())
        .map_err(|e| internal_error("load the PSKs of the MLS-group", e))?
        .build(
            state.provider.rand(),
            state.provider.crypto(),
            &identity.signer,
            |_| true,
        )
        .map_err(|e| internal_error("build the commit of the MLS-group", e))?
        .stage_commit(&state.provider)
        .map_err(|e| internal_error("stage the commit of the MLS-group", e))?;
    let (commit, welcome, _) = bundle.into_messages();

    let group_id = String::from_utf8_lossy(group.group_id().as_slice()).into_owned();
    let epoch = group.epoch().as_u64() + 1;

    let mut messages = Vec::new();
    if !recipients.is_empty() {
        messages.push(OutgoingMessage {
            group_id: group_id.clone(),
            epoch,
            message_type: MlsMessageType::Commit,
            recipients,
            payload: encode(&commit)?,
            grants: grants.clone(),
        });
    }
    if let Some(welcome) = welcome {
        messages.push(OutgoingMessage {
            group_id,
            epoch,
            message_type: MlsMessageType::Welcome,
            recipients: invited,
            payload: encode(&welcome)?,
            grants,
        });
    }
    Ok(messages)
}

/// Checks, that a change can be made: it has to change something, must neither remove nor add the
/// committer itself, and every removed gateway has to be a member.
///
/// # Arguments
/// * `group` - The group
/// * `vni` - Tenant of the network
/// * `own_client_id` - Identity of this gateway, which makes the change
/// * `additions` - Gateways to add
/// * `removals` - Identities of the gateways to remove
///
/// # Returns
/// `Ok(())`, `BadRequest` or `NotFound` with the reason, why the change can't be made
fn validate_change(
    group: &MlsGroup,
    vni: u32,
    own_client_id: &str,
    additions: &[Addition],
    removals: &[String],
) -> Result<(), ErrorResponse> {
    if additions.is_empty() && removals.is_empty() {
        return Err(ErrorResponse::BadRequest(
            "A change has to add or remove a gateway".to_string(),
        ));
    }
    if removals.iter().any(|it| it == own_client_id)
        || additions.iter().any(|it| it.client_id == own_client_id)
    {
        return Err(ErrorResponse::BadRequest(
            "A gateway can't remove or add itself, delete its group instead".to_string(),
        ));
    }
    for client_id in removals {
        if !group
            .members()
            .any(|member| identity_of(&member.credential).as_ref() == Some(client_id))
        {
            return Err(ErrorResponse::NotFound(format!(
                "'{client_id}' is not a member of the MLS-group of tenant {vni}"
            )));
        }
    }
    Ok(())
}

/// Lists the leaves of the members of a group with the given identities.
fn leaves_of(group: &MlsGroup, identities: &[&str]) -> Vec<LeafNodeIndex> {
    group
        .members()
        .filter(|member| {
            identity_of(&member.credential).is_some_and(|id| identities.contains(&id.as_str()))
        })
        .map(|member| member.index)
        .collect()
}

/// Stages a key-rotation of the group of a network: the gateway renews its own leaf, which moves
/// the group into a new epoch with a fresh secret.
///
/// # Arguments
/// * `state` - The MLS-client, which already has its identity
/// * `vni` - Tenant of the network
///
/// # Returns
/// The commit for the other members, or `None` if there are no other ones
pub fn stage_update(
    state: &mut MlsState,
    vni: u32,
) -> Result<Option<OutgoingMessage>, ErrorResponse> {
    let identity = own_identity(&state.identity)?;
    let group = state
        .groups
        .get_mut(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;

    let recipients = other_members(group, &identity.client_id, &[]);
    let bundle = group
        .self_update(
            &state.provider,
            &identity.signer,
            LeafNodeParameters::default(),
        )
        .map_err(|e| internal_error("rotate the keys of the MLS-group", e))?;

    if recipients.is_empty() {
        return Ok(None);
    }
    Ok(Some(OutgoingMessage {
        group_id: String::from_utf8_lossy(group.group_id().as_slice()).into_owned(),
        epoch: group.epoch().as_u64() + 1,
        message_type: MlsMessageType::Commit,
        recipients,
        payload: encode(bundle.commit())?,
        grants: Vec::new(),
    }))
}

/// Moves the group of a network into the epoch of its staged change.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// `Ok(())` once the group is in its new epoch
pub fn merge_pending(state: &mut MlsState, vni: u32) -> Result<(), ErrorResponse> {
    let group = state
        .groups
        .get_mut(&vni)
        .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
    group
        .merge_pending_commit(&state.provider)
        .map_err(|e| internal_error("merge the staged change of the MLS-group", e))
}

/// Drops the staged change of the group of a network, after it couldn't be delivered.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
pub fn clear_pending(state: &mut MlsState, vni: u32) {
    if let Some(group) = state.groups.get_mut(&vni)
        && let Err(e) = group.clear_pending_commit(state.provider.storage())
    {
        log::error!("Failed to drop the staged change of the MLS-group of tenant {vni}: {e}");
    }
}

/// The gateways, which a commit adds and removes, as the content of the commit tells it. MLS
/// authenticates the content, so izakaya can't change it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommitChange {
    /// Identity of the member, which made the commit
    pub committer: Option<String>,
    /// Identities of the added gateways, including the re-added ones
    pub added: Vec<String>,
    /// Identities of the removed gateways, including the re-added ones, whose old leaf goes
    pub removed: Vec<String>,
}

impl CommitChange {
    /// Lists the gateways, which the commit removes for good.
    pub fn gone(&self) -> Vec<&String> {
        self.removed
            .iter()
            .filter(|removed| !self.added.contains(removed))
            .collect()
    }

    /// Checks, if the commit has to move every pair to its new epoch: because it removes a
    /// gateway, which must not use any of the keys afterwards, or because it adds and removes
    /// nobody, which is the regular key-rotation. A commit, which only adds gateways, only moves
    /// their pairs, so a new gateway can't derive the keys between the others.
    pub fn moves_all(&self) -> bool {
        !self.gone().is_empty() || (self.added.is_empty() && self.removed.is_empty())
    }
}

/// Reads the gateways, which a staged commit adds and removes.
///
/// # Arguments
/// * `group` - The group, before the commit is merged
/// * `staged_commit` - The commit
/// * `committer` - Identity of the member, which made the commit
///
/// # Returns
/// The change of the commit
pub(crate) fn commit_change(
    group: &MlsGroup,
    staged_commit: &StagedCommit,
    committer: Option<String>,
) -> CommitChange {
    let mut change = CommitChange {
        committer,
        added: staged_commit
            .add_proposals()
            .filter_map(|addition| {
                identity_of(
                    addition
                        .add_proposal()
                        .key_package()
                        .leaf_node()
                        .credential(),
                )
            })
            .collect(),
        removed: staged_commit
            .remove_proposals()
            .filter_map(|removal| group.member(removal.remove_proposal().removed()))
            .filter_map(identity_of)
            .collect(),
    };
    change.added.sort();
    change.added.dedup();
    change.removed.sort();
    change.removed.dedup();
    change
}

/// Reads the gateways, which the staged change of this gateway adds and removes.
///
/// # Arguments
/// * `state` - The MLS-client
/// * `vni` - Tenant of the network
///
/// # Returns
/// The change, or a pure self-update, if nothing is staged
pub fn pending_change(state: &MlsState, vni: u32) -> CommitChange {
    let committer = state.client_id().map(str::to_string);
    match state
        .groups
        .get(&vni)
        .and_then(|group| group.pending_commit().map(|commit| (group, commit)))
    {
        Some((group, commit)) => commit_change(group, commit, committer),
        None => CommitChange {
            committer,
            ..Default::default()
        },
    }
}

/// Checks the grant of a gateway, which should be added to the group of a network.
///
/// # Arguments
/// * `grant` - The grant
/// * `grant_key` - Public key of hanami
/// * `vni` - Tenant of the network
/// * `client_id` - Identity of the gateway
///
/// # Returns
/// The content of the grant, or the reason, why it doesn't allow the gateway to join
pub fn verify_grant(
    grant: &MlsGrant,
    grant_key: &VerifyingKey,
    vni: u32,
    client_id: &str,
) -> Result<MlsGrantPayload, String> {
    let payload = grant.verify(grant_key)?;
    if payload.action != MlsGrantAction::Add {
        return Err("the grant doesn't allow to join".to_string());
    }
    if payload.vni != vni || payload.client_id != client_id {
        return Err(format!(
            "the grant is for '{}' in tenant {}",
            payload.client_id, payload.vni
        ));
    }
    Ok(payload)
}

/// Checks, that every gateway, which a commit adds, is covered by a grant of hanami.
///
/// The grant has to name the identity and the signature-key of the added member. Its expiration
/// is not checked here, because a member can't tell, when the committer made the commit.
///
/// # Arguments
/// * `staged_commit` - The commit
/// * `vni` - Tenant of the network
/// * `grants` - Grants, which are attached to the commit
/// * `grant_key` - Public key of hanami
///
/// # Returns
/// `Ok(())` if all additions are allowed, otherwise the first one, which is not
pub fn verify_additions(
    staged_commit: &StagedCommit,
    vni: u32,
    grants: &[MlsGrant],
    grant_key: &VerifyingKey,
) -> Result<(), String> {
    for addition in staged_commit.add_proposals() {
        let leaf = addition.add_proposal().key_package().leaf_node();
        let identity = identity_of(leaf.credential())
            .ok_or_else(|| "an added member has no basic credential".to_string())?;
        let signature_key = BASE64.encode(leaf.signature_key().as_slice());

        let covered = grants.iter().any(|grant| {
            verify_grant(grant, grant_key, vni, &identity)
                .is_ok_and(|payload| payload.signature_key == signature_key)
        });
        if !covered {
            return Err(format!("'{identity}' is added without a grant of hanami"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::mls_key_exchange::group::{
        create_group, create_key_packages, delete_group, is_member, members,
    };
    use crate::core::mls_key_exchange::test_support::{
        addition, deliver, grant_for, grant_key, invite, new_gateway, secret,
    };

    #[test]
    fn a_removed_gateway_loses_the_secret() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        invite(&mut a, &mut c, &mut [&mut b], 5);
        let old_secret = secret(&c, 5);

        let messages = stage_change(&mut a, 5, Vec::new(), &["10.0.0.3".to_string()]).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].recipients, vec!["10.0.0.2"]);
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut []);

        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_ne!(secret(&a, 5), old_secret);
        assert_eq!(members(&a, 5), vec!["10.0.0.1", "10.0.0.2"]);
        assert!(!is_member(&a, 5, "10.0.0.3"));
    }

    #[test]
    fn a_re_add_is_one_commit() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        invite(&mut a, &mut c, &mut [&mut b], 5);
        let epoch = a.groups[&5].epoch().as_u64();

        delete_group(&mut c, 5).unwrap();
        let addition = addition(&a, &c, 5);
        let messages = stage_change(&mut a, 5, vec![addition], &[]).unwrap();
        // the commit only goes to the remaining member, the re-added gateway gets the welcome
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].message_type, MlsMessageType::Commit);
        assert_eq!(messages[0].recipients, vec!["10.0.0.2"]);
        assert_eq!(messages[1].message_type, MlsMessageType::Welcome);
        assert_eq!(messages[1].recipients, vec!["10.0.0.3"]);
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut [&mut c]);

        assert_eq!(a.groups[&5].epoch().as_u64(), epoch + 1);
        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_eq!(secret(&a, 5), secret(&c, 5));
        assert_eq!(a.groups[&5].members().count(), 3);
    }

    #[test]
    fn several_gateways_are_added_with_one_commit() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        let mut d = new_gateway("10.0.0.4");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        let epoch = a.groups[&5].epoch().as_u64();

        let additions = vec![addition(&a, &c, 5), addition(&a, &d, 5)];
        let messages = stage_change(&mut a, 5, additions, &[]).unwrap();
        assert_eq!(messages[0].grants.len(), 2);
        assert_eq!(messages[1].recipients, vec!["10.0.0.3", "10.0.0.4"]);
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut [&mut c, &mut d]);

        assert_eq!(a.groups[&5].epoch().as_u64(), epoch + 1);
        for gateway in [&b, &c, &d] {
            assert_eq!(secret(&a, 5), secret(gateway, 5));
        }
        assert_eq!(
            members(&d, 5),
            vec!["10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.4"]
        );
    }

    #[tokio::test]
    async fn a_gateway_without_key_package_does_not_stop_the_others() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        let mut c = new_gateway("10.0.0.3");
        let d = new_gateway("10.0.0.4");
        let mut e = new_gateway("10.0.0.5");
        let f = new_gateway("10.0.0.6");
        let impostor = new_gateway("10.0.0.6");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);

        // d has no key-package left, the one of f was made by an impostor
        let mut key_packages = std::collections::HashMap::new();
        for gateway in [&c, &e, &impostor] {
            let client_id = gateway.client_id().unwrap().to_string();
            key_packages.insert(
                client_id,
                create_key_packages(gateway, 1).unwrap().remove(0),
            );
        }
        let adds: Vec<MlsOperationAdd> = [&c, &d, &e, &f]
            .into_iter()
            .map(|gateway| MlsOperationAdd {
                client_id: gateway.client_id().unwrap().to_string(),
                grant: grant_for(gateway, 5),
            })
            .collect();
        let claim = |target: String, _signature_key: String| {
            let key_package = key_packages.get(&target).cloned();
            async move { key_package.ok_or_else(|| "no key-package left".to_string()) }
        };
        let (additions, refused) =
            check_additions(&a, 5, &adds, &grant_key().verifying_key(), 0, claim).await;

        let refused: Vec<&str> = refused.iter().map(|it| it.client_id.as_str()).collect();
        assert_eq!(refused, vec!["10.0.0.4", "10.0.0.6"]);
        let messages = stage_change(&mut a, 5, additions, &[]).unwrap();
        merge_pending(&mut a, 5).unwrap();
        deliver(&messages, &mut b, &mut [&mut c, &mut e]);

        for gateway in [&b, &c, &e] {
            assert_eq!(secret(&a, 5), secret(gateway, 5));
        }
        assert_eq!(
            members(&a, 5),
            vec!["10.0.0.1", "10.0.0.2", "10.0.0.3", "10.0.0.5"]
        );
    }

    #[tokio::test]
    async fn an_expired_grant_is_refused_before_a_key_package_is_claimed() {
        let a = new_gateway("10.0.0.1");
        let c = new_gateway("10.0.0.3");
        let mut payload = grant_for(&c, 5)
            .verify(&grant_key().verifying_key())
            .unwrap();
        payload.expires_at = 100;
        let adds = vec![MlsOperationAdd {
            client_id: "10.0.0.3".to_string(),
            grant: MlsGrant::sign(&payload, &grant_key()),
        }];

        let claim = |_: String, _: String| async { panic!("no key-package is claimed") };
        let (additions, refused) =
            check_additions(&a, 5, &adds, &grant_key().verifying_key(), 101, claim).await;
        assert!(additions.is_empty());
        assert_eq!(refused[0].reason, "the grant is expired");
    }

    #[test]
    fn only_a_removal_for_good_or_a_self_update_moves_all_pairs() {
        let change = |added: &[&str], removed: &[&str]| CommitChange {
            committer: None,
            added: added.iter().map(|it| it.to_string()).collect(),
            removed: removed.iter().map(|it| it.to_string()).collect(),
        };
        assert!(change(&[], &[]).moves_all());
        assert!(change(&[], &["c"]).moves_all());
        assert!(change(&["d"], &["c"]).moves_all());
        assert!(!change(&["c"], &[]).moves_all());
        // a re-add only moves the pairs of the re-added gateway
        assert!(!change(&["c"], &["c"]).moves_all());
    }

    #[test]
    fn a_change_can_neither_be_empty_nor_remove_the_committer() {
        let mut a = new_gateway("10.0.0.1");
        create_group(&mut a, 5).unwrap();

        assert!(matches!(
            stage_change(&mut a, 5, Vec::new(), &[]),
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(matches!(
            stage_change(&mut a, 5, Vec::new(), &["10.0.0.1".to_string()]),
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(matches!(
            stage_change(&mut a, 5, Vec::new(), &["10.0.0.9".to_string()]),
            Err(ErrorResponse::NotFound(_))
        ));
    }

    #[test]
    fn a_grant_only_covers_its_gateway_and_tenant() {
        let c = new_gateway("10.0.0.3");
        let grant = grant_for(&c, 5);
        let key = grant_key().verifying_key();

        assert!(verify_grant(&grant, &key, 5, "10.0.0.3").is_ok());
        assert!(verify_grant(&grant, &key, 6, "10.0.0.3").is_err());
        assert!(verify_grant(&grant, &key, 5, "10.0.0.4").is_err());
    }

    #[test]
    fn a_self_update_rotates_the_secret_of_all_members() {
        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        create_group(&mut a, 5).unwrap();
        invite(&mut a, &mut b, &mut [], 5);
        let old_secret = secret(&a, 5);

        let commit = stage_update(&mut a, 5).unwrap().unwrap();
        merge_pending(&mut a, 5).unwrap();
        deliver(&[commit], &mut b, &mut []);

        assert_ne!(secret(&a, 5), old_secret);
        assert_eq!(secret(&a, 5), secret(&b, 5));
        assert_eq!(members(&a, 5), vec!["10.0.0.1", "10.0.0.2"]);
    }
}
