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
//! the database after every change, together with two entries of the gateway itself: the
//! identity and the list of its groups.

use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use openmls::prelude::*;
use openmls_basic_credential::SignatureKeyPair;
use openmls_rust_crypto::OpenMlsRustCrypto;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::config::CONFIG;
use crate::core::mls_key_exchange::keys::derive_base;
use crate::core::utils::get_local_ip;
use crate::database::mls_storage_table;

use ainari_api::errors::ErrorResponse;

/// Ciphersuite of all groups. It is the one every MLS-implementation has to support.
pub const CIPHERSUITE: Ciphersuite = Ciphersuite::MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519;

/// Entry of the key-value-store, which holds the identity of the gateway. The keys of openmls
/// itself are prefixed by their own labels, so they never collide with this one.
const IDENTITY_STORAGE_KEY: &[u8] = b"ainari/mls/identity";

/// Entry of the key-value-store, which lists the tenants the gateway has a group for
const GROUPS_STORAGE_KEY: &[u8] = b"ainari/mls/groups";

/// Entry of the key-value-store, which holds the key material of the epochs of all networks
const NETWORK_KEYS_STORAGE_KEY: &[u8] = b"ainari/mls/network_keys";

/// Prefix of the id of a group, which is followed by the tenant of the network
const GROUP_ID_PREFIX: &str = "ainari-vni-";

lazy_static::lazy_static! {
    pub static ref MLS_STATE_HANDLE: Arc<Mutex<MlsState>> = Arc::new(Mutex::new(MlsState::load()));
}

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

/// Persisted form of the identity of the gateway. The private key of the signature-key-pair is
/// stored by openmls itself and found again by its public key.
#[derive(Debug, Serialize, Deserialize)]
struct StoredIdentity {
    client_id: String,
    signature_public_key: String,
}

/// Identity of the gateway within all of its groups
pub struct MlsIdentity {
    /// Identity, which is written into the credential: the underlay-address of the gateway
    pub client_id: String,
    pub signer: SignatureKeyPair,
    pub credential: CredentialWithKey,
}

/// Id of an epoch of a network on this gateway: the generation of the group and the epoch within
/// the group. The generation counts the groups of the network, which the gateway created or
/// joined, so the epochs of a newer group are always newer than the ones of an older group.
pub type EpochId = (u64, u64);

/// The keys between this gateway and one peer gateway in a network. All connections between the
/// VMs of the two gateways change their keys together.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PairKeys {
    /// Epoch of the outgoing keys towards the peer, `None` before the first switch
    pub egress: Option<EpochId>,
    /// Epochs of the incoming keys from the peer
    pub ingress: BTreeSet<EpochId>,
    /// Epoch, which the pair is moving to
    pub target: Option<EpochId>,
}

/// The epochs of a network, whose IPsec keys are installed on the gateway.
///
/// openmls can only export secrets of the current epoch of a group, but a key-rotation needs
/// the keys of two epochs at the same time: the incoming keys of the new epoch are installed
/// first, while the outgoing traffic still uses the old epoch, until the peer can receive with
/// the new one. So a base secret is exported from every epoch, as soon as the gateway enters it,
/// and the keys of the connections are derived from that base secret.
///
/// The rotation runs in three steps, which the control plane triggers on all gateways of the
/// network one after another:
///
/// 1. a new epoch is recorded: it becomes the target of every pair and its incoming keys are
///    installed next to the old ones
/// 2. it is activated: the outgoing traffic of every pair switches over to its keys
/// 3. the older epochs are retired: their incoming keys are removed
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct NetworkKeys {
    /// Generation of the current group of the network
    pub generation: u64,
    /// Base secrets by epoch, kept as long as a pair uses the epoch
    #[serde(with = "epoch_map")]
    pub bases: BTreeMap<EpochId, Vec<u8>>,
    /// The pairs of the network, by the identity (underlay-address) of the peer
    pub pairs: BTreeMap<Ipv4Addr, PairKeys>,
}

/// Stores a map with epoch ids as keys as a list, because JSON only knows strings as keys.
mod epoch_map {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::EpochId;

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<EpochId, Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<EpochId, Vec<u8>>, D::Error> {
        let entries = Vec::<(EpochId, Vec<u8>)>::deserialize(deserializer)?;
        Ok(entries.into_iter().collect())
    }
}

impl NetworkKeys {
    /// Starts the keys of a network with the first epoch of a group, which every pair uses right
    /// away.
    ///
    /// # Arguments
    /// * `epoch` - The first epoch
    /// * `base` - Base secret of the epoch
    /// * `peers` - The other members of the group
    ///
    /// # Returns
    /// The keys with the epoch as the only and active one
    pub fn new(epoch: u64, base: Vec<u8>, peers: &[Ipv4Addr]) -> Self {
        let id = (1, epoch);
        let pair = PairKeys {
            egress: Some(id),
            ingress: BTreeSet::from([id]),
            target: None,
        };
        NetworkKeys {
            generation: 1,
            bases: BTreeMap::from([(id, base)]),
            pairs: peers.iter().map(|peer| (*peer, pair.clone())).collect(),
        }
    }

    /// Checks, if an epoch of the current group is recorded.
    pub fn has_epoch(&self, epoch: u64) -> bool {
        self.bases.contains_key(&(self.generation, epoch))
    }

    /// Records a new epoch of the current group: it becomes the target of every pair and its
    /// incoming keys are installed, while the outgoing traffic stays with the old epochs.
    ///
    /// # Arguments
    /// * `epoch` - The new epoch
    /// * `base` - Base secret of the epoch
    /// * `peers` - The other members of the group. A peer without a pair gets one, which has no
    ///   outgoing keys until the switch.
    pub fn record(&mut self, epoch: u64, base: Vec<u8>, peers: &[Ipv4Addr]) {
        let id = (self.generation, epoch);
        self.bases.insert(id, base);
        for peer in peers {
            self.pairs.entry(*peer).or_default();
        }
        for pair in self.pairs.values_mut() {
            pair.ingress.insert(id);
            pair.target = Some(id);
        }
    }

    /// Records the first epoch of a new group of the network, which replaces the current group,
    /// for example after izakaya lost its database. It is the next generation, so the pairs keep
    /// the keys of the older group, until they switched to the new one.
    ///
    /// # Arguments
    /// * `epoch` - The first epoch of the new group
    /// * `base` - Base secret of the epoch
    /// * `peers` - The other members of the new group
    pub fn replace(&mut self, epoch: u64, base: Vec<u8>, peers: &[Ipv4Addr]) {
        self.generation += 1;
        self.record(epoch, base, peers);
    }

    /// Switches the outgoing traffic of every pair, which can receive with it, to the keys of a
    /// recorded epoch of the current group.
    ///
    /// # Arguments
    /// * `epoch` - The epoch to activate
    ///
    /// # Returns
    /// `Ok(())`, `BadRequest` if the epoch is not recorded or `Conflict` if a newer epoch is
    /// active already
    pub fn activate(&mut self, epoch: u64) -> Result<(), ErrorResponse> {
        let id = (self.generation, epoch);
        if !self.bases.contains_key(&id) {
            return Err(ErrorResponse::BadRequest(format!(
                "Epoch {epoch} is not known (yet)"
            )));
        }
        if let Some(newer) = self
            .pairs
            .values()
            .filter_map(|pair| pair.egress)
            .find(|egress| *egress > id)
        {
            return Err(ErrorResponse::Conflict(format!(
                "Epoch {} is active already, which is newer than {epoch}",
                newer.1
            )));
        }
        for pair in self.pairs.values_mut() {
            if pair.ingress.contains(&id) {
                pair.egress = Some(id);
                if pair.target.is_some_and(|target| target <= id) {
                    pair.target = None;
                }
            }
        }
        Ok(())
    }

    /// Forgets all epochs, which are older than the given one, so their incoming keys are
    /// removed, and the pairs of the peers, which are no members of the group anymore.
    ///
    /// # Arguments
    /// * `epoch` - The oldest epoch of the current group, which is kept
    /// * `peers` - The other members of the group
    ///
    /// # Returns
    /// `Ok(())`, or `Conflict` if the outgoing traffic of a pair still uses an older epoch
    pub fn retire(&mut self, epoch: u64, peers: &[Ipv4Addr]) -> Result<(), ErrorResponse> {
        let id = (self.generation, epoch);
        let still_sending = self
            .pairs
            .iter()
            .any(|(peer, pair)| peers.contains(peer) && pair.egress.is_some_and(|e| e < id));
        if still_sending {
            return Err(ErrorResponse::Conflict(format!(
                "An older epoch than {epoch} is still active, activate epoch {epoch} first"
            )));
        }
        self.pairs.retain(|peer, _| peers.contains(peer));
        for pair in self.pairs.values_mut() {
            pair.ingress.retain(|ingress| *ingress >= id);
        }
        self.drop_unused_bases();
        Ok(())
    }

    /// Drops the base secrets, which no pair uses anymore.
    fn drop_unused_bases(&mut self) {
        let used: BTreeSet<EpochId> = self
            .pairs
            .values()
            .flat_map(|pair| {
                pair.ingress
                    .iter()
                    .copied()
                    .chain(pair.egress)
                    .chain(pair.target)
            })
            .collect();
        self.bases.retain(|id, _| used.contains(id));
    }
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
    fn restore(
        &mut self,
        values: std::collections::HashMap<Vec<u8>, Vec<u8>>,
    ) -> Result<(), String> {
        let identity = values.get(IDENTITY_STORAGE_KEY).cloned();
        let group_list = values.get(GROUPS_STORAGE_KEY).cloned();
        let network_keys = values.get(NETWORK_KEYS_STORAGE_KEY).cloned();
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

    /// Records the current epoch of the group of a network, after the group entered it.
    ///
    /// The base secret of the epoch is exported, so its incoming keys can be installed. The
    /// outgoing traffic stays with the active epochs, until the epoch is activated. Only the first
    /// epoch of a network, which has no keys yet, becomes the active one immediately.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    ///
    /// # Returns
    /// The recorded epoch, or an `InternalError`
    pub fn record_epoch(&mut self, vni: u32) -> Result<u64, ErrorResponse> {
        let (epoch, base) = self.current_base(vni)?;
        let peers = self.peers(vni);
        match self.network_keys.get_mut(&vni) {
            Some(keys) => keys.record(epoch, base, &peers),
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
            Some(keys) => keys.replace(epoch, base, &peers),
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
        self.network_keys.remove(&vni);
        self.record_epoch(vni)
    }

    /// Switches the outgoing traffic of a network to the keys of a recorded epoch.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    /// * `epoch` - The epoch to activate
    ///
    /// # Returns
    /// `Ok(())`, `NotFound` if the network has no keys, `BadRequest` if the epoch is not
    /// recorded or `Conflict` if a newer epoch is active already
    pub fn activate_epoch(&mut self, vni: u32, epoch: u64) -> Result<(), ErrorResponse> {
        self.network_keys
            .get_mut(&vni)
            .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?
            .activate(epoch)
    }

    /// Forgets all epochs of a network, which are older than the given one, so their incoming
    /// keys are removed, together with the pairs of the peers, which left the group.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    /// * `epoch` - The oldest epoch, which is kept
    ///
    /// # Returns
    /// `Ok(())`, `NotFound` if the network has no keys or `Conflict` if the outgoing traffic
    /// still uses an older epoch
    pub fn retire_epochs(&mut self, vni: u32, epoch: u64) -> Result<(), ErrorResponse> {
        let peers = self.peers(vni);
        self.network_keys
            .get_mut(&vni)
            .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?
            .retire(epoch, &peers)
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
fn identity_outdated(client_id: &str, underlay_ip: Option<Ipv4Addr>) -> bool {
    underlay_ip.is_some_and(|ip| client_id != ip.to_string())
}

/// Builds the identity of the gateway from its signature-key-pair.
///
/// # Arguments
/// * `client_id` - Identity, which is written into the credential
/// * `signer` - Signature-key-pair of the gateway
///
/// # Returns
/// The identity
fn new_identity(client_id: String, signer: SignatureKeyPair) -> MlsIdentity {
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

    /// The peer of the pairs within the tests
    const PEER: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 9);

    /// Builds the epochs of a network with one pair, which sends with `active_epoch`.
    fn network_keys(active_epoch: u64, epochs: &[u64]) -> NetworkKeys {
        NetworkKeys {
            generation: 1,
            bases: epochs
                .iter()
                .map(|epoch| ((1, *epoch), vec![0u8; 32]))
                .collect(),
            pairs: BTreeMap::from([(
                PEER,
                PairKeys {
                    egress: Some((1, active_epoch)),
                    ingress: epochs.iter().map(|epoch| (1, *epoch)).collect(),
                    target: epochs.last().map(|epoch| (1, *epoch)),
                },
            )]),
        }
    }

    #[test]
    fn only_a_recorded_epoch_can_be_activated() {
        let mut keys = network_keys(1, &[1, 2]);

        // an epoch, which the gateway didn't follow yet, would leave it without the key
        assert!(matches!(
            keys.activate(3),
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(keys.activate(2).is_ok());
        assert_eq!(keys.pairs[&PEER].egress, Some((1, 2)));
        assert_eq!(keys.pairs[&PEER].target, None);
        // activating the active epoch again changes nothing, an older one is refused
        assert!(keys.activate(2).is_ok());
        assert!(matches!(keys.activate(1), Err(ErrorResponse::Conflict(_))));

        let mut state = MlsState::new_in_memory();
        assert!(matches!(
            state.activate_epoch(6, 1),
            Err(ErrorResponse::NotFound(_))
        ));
    }

    #[test]
    fn only_the_epochs_before_the_active_one_can_be_retired() {
        let mut keys = network_keys(2, &[1, 2, 3]);

        // epoch 2 is still used for the outgoing traffic
        assert!(matches!(
            keys.retire(3, &[PEER]),
            Err(ErrorResponse::Conflict(_))
        ));
        assert!(keys.retire(2, &[PEER]).is_ok());
        let epochs = |keys: &NetworkKeys| keys.bases.keys().copied().collect::<Vec<_>>();
        assert_eq!(epochs(&keys), vec![(1, 2), (1, 3)]);
        assert_eq!(keys.pairs[&PEER].ingress, BTreeSet::from([(1, 2), (1, 3)]));
        // retiring again changes nothing
        assert!(keys.retire(2, &[PEER]).is_ok());
        assert_eq!(epochs(&keys), vec![(1, 2), (1, 3)]);
    }

    #[test]
    fn a_new_epoch_is_recorded_without_being_activated() {
        use crate::core::mls_key_exchange::group::tests::{invite, new_gateway};

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
        let second = a.record_epoch(7).unwrap();

        assert_eq!(second, first + 1);
        let keys = &a.network_keys[&7];
        let pair = &keys.pairs[&Ipv4Addr::new(10, 0, 0, 2)];
        assert_eq!(pair.egress, Some((1, first)));
        assert_eq!(pair.target, Some((1, second)));
        assert_eq!(pair.ingress, BTreeSet::from([(1, first), (1, second)]));
        assert_eq!(keys.pairs.len(), 1);
    }

    #[test]
    fn a_replaced_group_keeps_the_old_keys_until_the_switch() {
        let mut keys = network_keys(2, &[2]);
        let other = Ipv4Addr::new(10, 0, 0, 8);

        // the new group has a new member, the old peer joins it as well
        keys.replace(0, vec![1u8; 32], &[PEER, other]);
        assert_eq!(keys.generation, 2);
        assert_eq!(keys.pairs[&PEER].egress, Some((1, 2)));
        assert_eq!(keys.pairs[&PEER].target, Some((2, 0)));
        assert_eq!(keys.pairs[&other].egress, None);

        // the epochs of the new group are newer than the ones of the old group
        keys.activate(0).unwrap();
        keys.retire(0, &[PEER, other]).unwrap();
        assert_eq!(keys.pairs[&PEER].egress, Some((2, 0)));
        assert_eq!(keys.pairs[&PEER].ingress, BTreeSet::from([(2, 0)]));
        assert_eq!(keys.pairs[&other].egress, Some((2, 0)));
        assert_eq!(keys.bases.keys().copied().collect::<Vec<_>>(), vec![(2, 0)]);
    }

    #[test]
    fn the_key_state_survives_a_restore() {
        let keys = network_keys(1, &[1, 2]);
        let json = serde_json::to_vec(&keys).unwrap();
        let restored: NetworkKeys = serde_json::from_slice(&json).unwrap();
        assert_eq!(restored.generation, keys.generation);
        assert_eq!(restored.bases, keys.bases);
        assert_eq!(restored.pairs, keys.pairs);
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
