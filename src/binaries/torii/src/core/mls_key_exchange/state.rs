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

//! State of the MLS-client of the gateway.
//!
//! The gateway is one client with one identity, which is its underlay-address, and it is a
//! member of one group for each network, which has a VM on this gateway. Everything openmls
//! knows about the client lives in the key-value-store of the provider, which is persisted into
//! the database after every change, together with the entries of the gateway itself: the
//! identity, the list of its groups, the key state of its networks and the messages, which wait to
//! be sent or for their epoch.

use crate::config::CONFIG;
use crate::core::mls_key_exchange::commit::CommitChange;
use crate::core::mls_key_exchange::identity::{CIPHERSUITE, MlsIdentity, group_id_of, identity_of};
use crate::core::mls_key_exchange::identity::{identity_outdated, new_identity};
use crate::core::mls_key_exchange::keys::derive_base;
use crate::core::mls_key_exchange::messages::OutgoingMessage;
use crate::core::mls_key_exchange::network_keys::NetworkKeys;
use crate::core::utils::get_local_ip;
use crate::database::mls_storage_table;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::MlsMessageResp;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Entry of the key-value-store, which holds the identity of the gateway. The keys of openmls
/// itself are prefixed by their own labels, so they never collide with this one.
const IDENTITY_STORAGE_KEY: &[u8] = b"ainari/mls/identity";

/// Entry of the key-value-store, which lists the tenants the gateway has a group for
const GROUPS_STORAGE_KEY: &[u8] = b"ainari/mls/groups";

/// Entry of the key-value-store, which holds the key material of the epochs of all networks
const NETWORK_KEYS_STORAGE_KEY: &[u8] = b"ainari/mls/network_keys";

/// Entry of the key-value-store, which holds the messages, which izakaya didn't accept yet
const PENDING_OUT_STORAGE_KEY: &[u8] = b"ainari/mls/pending_out";

/// Entry of the key-value-store, which holds the messages of epochs, which the groups didn't
/// reach yet
const PENDING_IN_STORAGE_KEY: &[u8] = b"ainari/mls/pending_in";

lazy_static::lazy_static! {
    pub static ref MLS_STATE_HANDLE: Arc<Mutex<MlsState>> = Arc::new(Mutex::new(MlsState::load()));
}

/// Persisted form of the identity of the gateway. The private key of the signature-key-pair is
/// stored by openmls itself and found again by its public key.
#[derive(Debug, Serialize, Deserialize)]
struct StoredIdentity {
    client_id: String,
    signature_public_key: String,
}

/// The MLS-client of the gateway with all of its groups
pub struct MlsState {
    pub provider: OpenMlsRustCrypto,
    /// Identity of the gateway, which is created with the first use by `ensure_identity`
    pub identity: Option<MlsIdentity>,
    /// The groups of the gateway, by the tenant of their network
    pub groups: BTreeMap<u32, MlsGroup>,
    /// The epochs of the networks, whose IPsec keys are installed, by the tenant of the network
    pub network_keys: BTreeMap<u32, NetworkKeys>,
    /// Messages to other gateways, which izakaya didn't accept yet, in the order of sending
    pub pending_out: Vec<OutgoingMessage>,
    /// Messages of other gateways, whose epoch the group didn't reach yet
    pub pending_in: Vec<MlsMessageResp>,
    /// Write every change into the database. Only switched off by the tests, which simulate
    /// several gateways within one process.
    persist: bool,
}

impl MlsState {
    /// Creates a client without identity and groups, which isn't written into the database.
    pub fn new_in_memory() -> Self {
        MlsState {
            provider: OpenMlsRustCrypto::default(),
            identity: None,
            groups: BTreeMap::new(),
            network_keys: BTreeMap::new(),
            pending_out: Vec::new(),
            pending_in: Vec::new(),
            persist: false,
        }
    }

    /// Loads the client from the database.
    ///
    /// A broken state doesn't keep the gateway down: it is logged and the gateway starts without
    /// groups, so the control plane can invite it again.
    ///
    /// # Returns
    /// The restored client
    pub fn load() -> Self {
        let mut state = MlsState::new_in_memory();
        state.persist = true;

        let values = match mls_storage_table::load_mls_storage() {
            Ok(values) => values,
            Err(e) => {
                log::error!("Failed to load the MLS-state, starting without groups: {e}");
                return state;
            }
        };
        if let Err(e) = state.restore(values) {
            log::error!("Failed to restore the MLS-state, starting without groups: {e}");
            let mut state = MlsState::new_in_memory();
            state.persist = true;
            return state;
        }

        // The identity is the underlay-address of the gateway. A gateway, which came back on
        // another address, for example in a new pod, whose database was kept, can't use its old
        // identity anymore: hanami only grants the identity, which matches the address of the
        // host. So it starts with a new identity and without groups, and joins the groups again
        // with the grants for its new identity. Its old identity is removed from the groups.
        let underlay_ip = get_local_ip(&CONFIG.network.underlay_iface);
        if let Some(previous) = state.client_id()
            && identity_outdated(previous, underlay_ip)
        {
            log::warn!(
                "The MLS-identity '{previous}' belongs to another underlay address than {}, so \
                 the gateway starts with a new identity and without groups",
                underlay_ip.map(|ip| ip.to_string()).unwrap_or_default()
            );
            let mut fresh = MlsState::new_in_memory();
            fresh.persist = true;
            if let Err(e) = fresh.save() {
                log::error!("Failed to drop the outdated MLS-state: {e:?}");
            }
            return fresh;
        }

        log::info!(
            "Restored MLS-client '{}' with {} group(s)",
            state.client_id().unwrap_or("-"),
            state.groups.len()
        );
        state
    }

    /// Fills the client from a key-value-store, which was persisted by `save`.
    ///
    /// # Arguments
    /// * `values` - The persisted key-value-store
    ///
    /// # Returns
    /// `Ok(())` if the identity and all groups could be restored, otherwise the reason why not
    pub(crate) fn restore(
        &mut self,
        values: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<(), String> {
        let identity = values.get(IDENTITY_STORAGE_KEY).cloned();
        let group_list = values.get(GROUPS_STORAGE_KEY).cloned();
        let network_keys = values.get(NETWORK_KEYS_STORAGE_KEY).cloned();
        let pending_out = values.get(PENDING_OUT_STORAGE_KEY).cloned();
        let pending_in = values.get(PENDING_IN_STORAGE_KEY).cloned();
        *self
            .provider
            .storage()
            .values
            .write()
            .map_err(|_| "mls-storage poisoned".to_string())? = values;

        if let Some(identity) = identity {
            let stored: StoredIdentity =
                serde_json::from_slice(&identity).map_err(|e| format!("broken identity: {e}"))?;
            let public_key = BASE64
                .decode(&stored.signature_public_key)
                .map_err(|e| format!("broken public key of the identity: {e}"))?;
            let signer = SignatureKeyPair::read(
                self.provider.storage(),
                &public_key,
                CIPHERSUITE.signature_algorithm(),
            )
            .ok_or_else(|| "signature-key of the identity is missing".to_string())?;
            self.identity = Some(new_identity(stored.client_id, signer));
        }

        if let Some(group_list) = group_list {
            let vnis: Vec<u32> = serde_json::from_slice(&group_list)
                .map_err(|e| format!("broken list of groups: {e}"))?;
            for vni in vnis {
                match MlsGroup::load(self.provider.storage(), &group_id_of(vni)) {
                    Ok(Some(group)) => {
                        self.groups.insert(vni, group);
                    }
                    Ok(None) => log::warn!("MLS-group of tenant {vni} is missing in the storage"),
                    Err(e) => log::error!("Failed to load the MLS-group of tenant {vni}: {e}"),
                }
            }
        }

        if let Some(network_keys) = network_keys {
            self.network_keys = serde_json::from_slice(&network_keys)
                .map_err(|e| format!("broken key material of the networks: {e}"))?;
        }
        if let Some(pending_out) = pending_out {
            self.pending_out = serde_json::from_slice(&pending_out)
                .map_err(|e| format!("broken outgoing messages: {e}"))?;
        }
        if let Some(pending_in) = pending_in {
            self.pending_in = serde_json::from_slice(&pending_in)
                .map_err(|e| format!("broken waiting messages: {e}"))?;
        }

        Ok(())
    }

    /// Writes the client into the database.
    ///
    /// # Returns
    /// `Ok(())` once the state is persisted, otherwise an `InternalError`
    pub fn save(&self) -> Result<(), ErrorResponse> {
        let internal_error = |e: String| {
            ainari_api::report_internal_error!(None, "Failed to persist the MLS-state: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        };

        let mut values = self
            .provider
            .storage()
            .values
            .write()
            .map_err(|_| internal_error("mls-storage poisoned".to_string()))?;

        if let Some(identity) = &self.identity {
            let stored = StoredIdentity {
                client_id: identity.client_id.clone(),
                signature_public_key: BASE64.encode(identity.signer.public()),
            };
            let stored = serde_json::to_vec(&stored).map_err(|e| internal_error(e.to_string()))?;
            values.insert(IDENTITY_STORAGE_KEY.to_vec(), stored);
        }
        let vnis: Vec<u32> = self.groups.keys().copied().collect();
        let vnis = serde_json::to_vec(&vnis).map_err(|e| internal_error(e.to_string()))?;
        values.insert(GROUPS_STORAGE_KEY.to_vec(), vnis);
        let network_keys =
            serde_json::to_vec(&self.network_keys).map_err(|e| internal_error(e.to_string()))?;
        values.insert(NETWORK_KEYS_STORAGE_KEY.to_vec(), network_keys);
        let pending_out =
            serde_json::to_vec(&self.pending_out).map_err(|e| internal_error(e.to_string()))?;
        values.insert(PENDING_OUT_STORAGE_KEY.to_vec(), pending_out);
        let pending_in =
            serde_json::to_vec(&self.pending_in).map_err(|e| internal_error(e.to_string()))?;
        values.insert(PENDING_IN_STORAGE_KEY.to_vec(), pending_in);

        if self.persist {
            mls_storage_table::save_mls_storage(&values)
                .map_err(|e| internal_error(e.to_string()))?;
        }
        Ok(())
    }

    /// Returns the identity of the gateway, if it already has one
    pub fn client_id(&self) -> Option<&str> {
        self.identity
            .as_ref()
            .map(|identity| identity.client_id.as_str())
    }

    /// Creates the identity of the gateway with the first use.
    ///
    /// The identity is the underlay-address of the gateway, because that is the address the
    /// control plane knows the gateway by: it is the `gateway_ip` of every route towards the VMs
    /// behind this gateway.
    ///
    /// # Returns
    /// The identity, or an `InternalError`, if the underlay has no address yet
    pub fn ensure_identity(&mut self) -> Result<&MlsIdentity, ErrorResponse> {
        if self.identity.is_none() {
            let client_id = match get_local_ip(&CONFIG.network.underlay_iface) {
                Some(ip) => ip.to_string(),
                None => {
                    ainari_api::report_internal_error!(
                        None,
                        "No underlay address on {}, so the gateway has no MLS-identity",
                        CONFIG.network.underlay_iface
                    );
                    return Err(ErrorResponse::InternalError("Internal Error".to_string()));
                }
            };
            self.create_identity(client_id)?;
        } else if let Some(ip) = get_local_ip(&CONFIG.network.underlay_iface)
            && self.client_id() != Some(ip.to_string().as_str())
        {
            log::warn!(
                "The MLS-identity '{}' doesn't match the underlay address {ip} anymore",
                self.client_id().unwrap_or("-")
            );
        }

        self.identity.as_ref().ok_or_else(|| {
            ainari_api::report_internal_error!(
                None,
                "The MLS-identity of the gateway was not created yet"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        })
    }

    /// Creates a new identity for the gateway and persists it.
    ///
    /// # Arguments
    /// * `client_id` - Identity, which is written into the credential
    ///
    /// # Returns
    /// `Ok(())` once the identity exists, otherwise an `InternalError`
    pub fn create_identity(&mut self, client_id: String) -> Result<(), ErrorResponse> {
        let internal_error = |action: &str, e: String| {
            ainari_api::report_internal_error!(None, "Failed to {action} of the MLS-identity: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        };

        let signer = SignatureKeyPair::new(CIPHERSUITE.signature_algorithm())
            .map_err(|e| internal_error("create the signature-key", format!("{e:?}")))?;
        signer
            .store(self.provider.storage())
            .map_err(|e| internal_error("store the signature-key", e.to_string()))?;

        log::info!("Created MLS-identity '{client_id}'");
        self.identity = Some(new_identity(client_id, signer));
        self.save()
    }

    /// Lists the other members of the group of a network, which are the peers of its pairs.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    ///
    /// # Returns
    /// The underlay-addresses of the other members, ordered
    pub fn peers(&self, vni: u32) -> Vec<Ipv4Addr> {
        let own = self.client_id();
        let Some(group) = self.groups.get(&vni) else {
            return Vec::new();
        };
        let mut peers: Vec<Ipv4Addr> = group
            .members()
            .filter_map(|member| identity_of(&member.credential))
            .filter(|id| Some(id.as_str()) != own)
            .filter_map(|id| id.parse().ok())
            .collect();
        peers.sort();
        peers.dedup();
        peers
    }

    /// Exports the base secret of the current epoch of the group of a network.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    ///
    /// # Returns
    /// The epoch and its base secret, or an `InternalError`
    fn current_base(&self, vni: u32) -> Result<(u64, Vec<u8>), ErrorResponse> {
        let group = self
            .groups
            .get(&vni)
            .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
        let epoch = group.epoch().as_u64();
        let base = derive_base(group, self.provider.crypto()).map_err(|e| {
            ainari_api::report_internal_error!(
                None,
                "Failed to record epoch {epoch} of tenant {vni}: {e}"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;
        Ok((epoch, base))
    }

    /// Records the epoch of the group of a network, which a commit moved it into.
    ///
    /// The base secret of the epoch is exported, so its incoming keys can be installed. Which
    /// pairs move to the epoch, only depends on the content of the commit (see
    /// `CommitChange::moves_all`): the pairs of the added gateways always, all others only with a
    /// removal or a key-rotation. The outgoing traffic of a pair stays with its active epoch, until
    /// the pair switched. Only the first epoch of a network, which has no keys yet, becomes the
    /// active one immediately.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    /// * `change` - The gateways, which the commit added and removed
    ///
    /// # Returns
    /// The recorded epoch, or an `InternalError`
    pub fn apply_commit(&mut self, vni: u32, change: &CommitChange) -> Result<u64, ErrorResponse> {
        let (epoch, base) = self.current_base(vni)?;
        let peers = self.peers(vni);
        let committer = change.committer.as_deref().unwrap_or("-");
        for gone in change.gone() {
            log::warn!("'{committer}' removed '{gone}' from the MLS-group of tenant {vni}");
        }
        let added: Vec<Ipv4Addr> = change
            .added
            .iter()
            .filter_map(|added| added.parse().ok())
            .collect();
        let moving = (!change.moves_all()).then_some(added.as_slice());

        match self.network_keys.get_mut(&vni) {
            Some(keys) => keys.record(epoch, base, &peers, moving, Utc::now().timestamp()),
            None => {
                self.network_keys
                    .insert(vni, NetworkKeys::new(epoch, base, &peers));
            }
        }
        Ok(epoch)
    }

    /// Records the first epoch of a new group of a network, which replaces an older group of the
    /// same network, for example after izakaya lost its database and the group was started
    /// again.
    ///
    /// The keys of the older group stay in use, until the first rotation of the new group
    /// switched the pairs over (see `NetworkKeys::replace`). Without an older group, the epoch
    /// becomes the only and active one.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    ///
    /// # Returns
    /// The recorded epoch, or an `InternalError`
    pub fn replace_epochs(&mut self, vni: u32) -> Result<u64, ErrorResponse> {
        let (epoch, base) = self.current_base(vni)?;
        let peers = self.peers(vni);
        match self.network_keys.get_mut(&vni) {
            Some(keys) => keys.replace(epoch, base, &peers, Utc::now().timestamp()),
            None => {
                self.network_keys
                    .insert(vni, NetworkKeys::new(epoch, base, &peers));
            }
        }
        Ok(epoch)
    }

    /// Forgets all epochs of a network and records the current epoch of its group as the only
    /// and active one. This is used for a group, which was created or joined right now, so the
    /// keys of an older group of the network are removed.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    ///
    /// # Returns
    /// The recorded epoch, or an `InternalError`
    pub fn reset_epochs(&mut self, vni: u32) -> Result<u64, ErrorResponse> {
        let (epoch, base) = self.current_base(vni)?;
        let peers = self.peers(vni);
        self.network_keys
            .insert(vni, NetworkKeys::new(epoch, base, &peers));
        Ok(epoch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::core::mls_key_exchange::commit::CommitChange;
    use crate::core::mls_key_exchange::test_support;

    #[test]
    fn the_identity_and_the_groups_survive_a_restore() {
        let mut state = MlsState::new_in_memory();
        state.create_identity("10.0.0.5".to_string()).unwrap();
        crate::core::mls_key_exchange::group::create_group(&mut state, 7).unwrap();
        state.reset_epochs(7).unwrap();
        state.save().unwrap();

        let values = state.provider.storage().values.read().unwrap().clone();
        let mut restored = MlsState::new_in_memory();
        restored.restore(values).unwrap();

        assert_eq!(restored.client_id(), Some("10.0.0.5"));
        assert_eq!(
            crate::core::mls_key_exchange::group::members(&restored, 7),
            vec!["10.0.0.5".to_string()]
        );
        let epoch = restored.groups[&7].epoch().as_u64();
        assert_eq!(epoch, state.groups[&7].epoch().as_u64());
        let keys = &restored.network_keys[&7];
        assert_eq!(
            keys.bases.keys().copied().collect::<Vec<_>>(),
            vec![(1, epoch)]
        );
        assert_eq!(keys.generation, 1);
        assert_eq!(
            restored.network_keys[&7].bases,
            state.network_keys[&7].bases
        );
        assert_eq!(
            restored.identity.as_ref().unwrap().signer.public(),
            state.identity.as_ref().unwrap().signer.public()
        );
    }

    #[test]
    fn a_new_epoch_is_recorded_without_being_activated() {
        use test_support::{invite, new_gateway};

        let mut a = new_gateway("10.0.0.1");
        let mut b = new_gateway("10.0.0.2");
        crate::core::mls_key_exchange::group::create_group(&mut a, 7).unwrap();
        invite(&mut a, &mut b, &mut [], 7);
        let first = a.reset_epochs(7).unwrap();

        // the group moves into the next epoch, for example by a self-update
        let identity = a.identity.as_ref().unwrap();
        let group = a.groups.get_mut(&7).unwrap();
        group
            .self_update(
                &a.provider,
                &identity.signer,
                openmls::prelude::LeafNodeParameters::default(),
            )
            .unwrap();
        group.merge_pending_commit(&a.provider).unwrap();
        let second = a.apply_commit(7, &CommitChange::default()).unwrap();

        assert_eq!(second, first + 1);
        let keys = &a.network_keys[&7];
        let pair = &keys.pairs[&Ipv4Addr::new(10, 0, 0, 2)];
        assert_eq!(pair.egress, Some((1, first)));
        assert_eq!(pair.target, Some((1, second)));
        assert_eq!(pair.ingress, BTreeSet::from([(1, first), (1, second)]));
        assert_eq!(keys.pairs.len(), 1);
    }
}
