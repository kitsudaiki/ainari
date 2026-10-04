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

use std::collections::BTreeMap;
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
use crate::core::mls::keys::derive_base;
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

/// The epochs of a network, whose IPsec keys are installed on the gateway.
///
/// openmls can only export secrets of the current epoch of a group, but a key-rotation needs
/// the keys of two epochs at the same time: the incoming keys of the new epoch are installed
/// first, while the outgoing traffic still uses the old epoch, until every gateway of the network
/// can receive with the new one. So a base secret is exported from every epoch, as soon as the
/// gateway enters it, and the keys of the routes are derived from that base secret.
///
/// The rotation runs in three steps, which the control plane triggers on all gateways of the
/// network one after another:
///
/// 1. a new epoch is recorded: its incoming keys are installed next to the old ones
/// 2. it is activated: the outgoing traffic switches over to its keys
/// 3. the older epochs are retired: their incoming keys are removed
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct NetworkKeys {
    /// Epoch, whose keys protect the outgoing traffic
    pub active_epoch: u64,
    /// Base secrets of all epochs, whose incoming keys are installed, by their epoch
    pub bases: BTreeMap<u64, Vec<u8>>,
    /// Keys of the group, which the current group of the network replaced, for example after
    /// izakaya lost its database. They stay in use, until the first rotation of the new group
    /// switched all members over, so the replacement doesn't lose a packet either.
    #[serde(default)]
    pub replaced: Option<ReplacedKeys>,
}

/// Keys of a replaced group of a network
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ReplacedKeys {
    /// Epoch of the replaced group, whose keys protect the outgoing traffic, while `sending` is set
    pub active_epoch: u64,
    /// Base secrets of the replaced group, whose incoming keys stay installed until the new group
    /// retires them
    pub bases: BTreeMap<u64, Vec<u8>>,
    /// The outgoing traffic still uses the replaced group, because the new group didn't switch
    /// yet
    pub sending: bool,
}

impl NetworkKeys {
    /// Starts the keys of a network with the first epoch of a group.
    ///
    /// # Arguments
    /// * `epoch` - The first epoch
    /// * `base` - Base secret of the epoch
    ///
    /// # Returns
    /// The keys with the epoch as the only and active one
    pub fn new(epoch: u64, base: Vec<u8>) -> Self {
        NetworkKeys {
            active_epoch: epoch,
            bases: BTreeMap::from([(epoch, base)]),
            replaced: None,
        }
    }

    /// Starts the keys of a new group of a network, which replaces the current group.
    ///
    /// The incoming keys of the current group stay installed and its outgoing keys stay in use,
    /// until the first epoch of the new group is activated. A group, which replaced another one
    /// itself and never sent, hands the keys it still sends with over.
    ///
    /// # Arguments
    /// * `epoch` - The first epoch of the new group
    /// * `base` - Base secret of the epoch
    ///
    /// # Returns
    /// The keys of the new group
    pub fn replaced_by(self, epoch: u64, base: Vec<u8>) -> Self {
        let replaced = match self.replaced {
            Some(mut older) if older.sending => {
                for (key_epoch, key_base) in self.bases {
                    older.bases.entry(key_epoch).or_insert(key_base);
                }
                older
            }
            _ => ReplacedKeys {
                active_epoch: self.active_epoch,
                bases: self.bases,
                sending: true,
            },
        };

        let mut keys = NetworkKeys::new(epoch, base);
        keys.replaced = Some(replaced);
        keys
    }

    /// Returns the epoch and the base secret, whose keys protect the outgoing traffic.
    pub fn egress(&self) -> Option<(u64, &Vec<u8>)> {
        match &self.replaced {
            Some(replaced) if replaced.sending => replaced
                .bases
                .get(&replaced.active_epoch)
                .map(|base| (replaced.active_epoch, base)),
            _ => self
                .bases
                .get(&self.active_epoch)
                .map(|base| (self.active_epoch, base)),
        }
    }

    /// Returns all epochs and base secrets, whose incoming keys are installed.
    pub fn ingress(&self) -> Vec<(u64, &Vec<u8>)> {
        let mut ingress: Vec<(u64, &Vec<u8>)> = self
            .bases
            .iter()
            .map(|(epoch, base)| (*epoch, base))
            .collect();
        if let Some(replaced) = &self.replaced {
            ingress.extend(replaced.bases.iter().map(|(epoch, base)| (*epoch, base)));
        }
        ingress
    }

    /// Switches the outgoing traffic to the keys of a recorded epoch of the current group.
    ///
    /// # Arguments
    /// * `epoch` - The epoch to activate
    ///
    /// # Returns
    /// `Ok(())`, `BadRequest` if the epoch is not recorded or `Conflict` if a newer epoch is
    /// active already
    pub fn activate(&mut self, epoch: u64) -> Result<(), ErrorResponse> {
        if !self.bases.contains_key(&epoch) {
            return Err(ErrorResponse::BadRequest(format!(
                "Epoch {epoch} is not known (yet)"
            )));
        }
        if epoch < self.active_epoch {
            return Err(ErrorResponse::Conflict(format!(
                "Epoch {} is active already, which is newer than {epoch}",
                self.active_epoch
            )));
        }
        self.active_epoch = epoch;
        if let Some(replaced) = &mut self.replaced {
            replaced.sending = false;
        }
        Ok(())
    }

    /// Forgets all epochs of the current group, which are older than the given one, and the
    /// replaced group, so their incoming keys are removed.
    ///
    /// # Arguments
    /// * `epoch` - The oldest epoch, which is kept
    ///
    /// # Returns
    /// `Ok(())`, or `Conflict` if the outgoing traffic still uses an older epoch
    pub fn retire(&mut self, epoch: u64) -> Result<(), ErrorResponse> {
        let replaced_sending = self
            .replaced
            .as_ref()
            .is_some_and(|replaced| replaced.sending);
        if self.active_epoch < epoch || replaced_sending {
            return Err(ErrorResponse::Conflict(format!(
                "An older epoch than {epoch} is still active, activate epoch {epoch} first"
            )));
        }
        self.bases.retain(|key_epoch, _| *key_epoch >= epoch);
        self.replaced = None;
        Ok(())
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
            log::error!("Failed to persist the MLS-state: {e}");
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
                    log::error!(
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

        self.identity
            .as_ref()
            .ok_or_else(|| ErrorResponse::InternalError("Internal Error".to_string()))
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
            log::error!("Failed to {action} of the MLS-identity: {e}");
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

    /// Records the current epoch of the group of a network, after the group entered it.
    ///
    /// The base secret of the epoch is exported, so its incoming keys can be installed. The
    /// outgoing traffic stays with the active epoch, until the epoch is activated. Only the first
    /// epoch of a network, which has no active one yet, becomes the active one immediately.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    ///
    /// # Returns
    /// The recorded epoch, or an `InternalError`
    pub fn record_epoch(&mut self, vni: u32) -> Result<u64, ErrorResponse> {
        let group = self
            .groups
            .get(&vni)
            .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?;
        let epoch = group.epoch().as_u64();
        let base = derive_base(group, self.provider.crypto()).map_err(|e| {
            log::error!("Failed to record epoch {epoch} of tenant {vni}: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

        match self.network_keys.get_mut(&vni) {
            Some(keys) => {
                keys.bases.insert(epoch, base);
            }
            None => {
                self.network_keys.insert(vni, NetworkKeys::new(epoch, base));
            }
        }
        Ok(epoch)
    }

    /// Records the first epoch of a new group of a network, which replaces an older group of the
    /// same network, for example after izakaya lost its database and the group was started
    /// again.
    ///
    /// The keys of the older group stay in use, until the first rotation of the new group
    /// switched all members over (see `NetworkKeys::replaced_by`). Without an older group, the
    /// epoch becomes the only and active one.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    ///
    /// # Returns
    /// The recorded epoch, or an `InternalError`
    pub fn replace_epochs(&mut self, vni: u32) -> Result<u64, ErrorResponse> {
        let Some(old) = self.network_keys.remove(&vni) else {
            return self.record_epoch(vni);
        };
        let epoch = self.record_epoch(vni)?;
        if let Some(new) = self.network_keys.remove(&vni) {
            let base = new.bases.get(&epoch).cloned().unwrap_or_default();
            self.network_keys.insert(vni, old.replaced_by(epoch, base));
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
    /// keys are removed.
    ///
    /// # Arguments
    /// * `vni` - Tenant of the network
    /// * `epoch` - The oldest epoch, which is kept
    ///
    /// # Returns
    /// `Ok(())`, `NotFound` if the network has no keys or `Conflict` if the outgoing traffic
    /// still uses an older epoch
    pub fn retire_epochs(&mut self, vni: u32, epoch: u64) -> Result<(), ErrorResponse> {
        self.network_keys
            .get_mut(&vni)
            .ok_or_else(|| ErrorResponse::NotFound(format!("No MLS-group for tenant {vni}")))?
            .retire(epoch)
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
        crate::core::mls::group::create_group(&mut state, 7).unwrap();
        state.reset_epochs(7).unwrap();
        state.save().unwrap();

        let values = state.provider.storage().values.read().unwrap().clone();
        let mut restored = MlsState::new_in_memory();
        restored.restore(values).unwrap();

        assert_eq!(restored.client_id(), Some("10.0.0.5"));
        assert_eq!(
            crate::core::mls::group::members(&restored, 7),
            vec!["10.0.0.5".to_string()]
        );
        let epoch = restored.groups[&7].epoch().as_u64();
        assert_eq!(epoch, state.groups[&7].epoch().as_u64());
        let keys = &restored.network_keys[&7];
        assert_eq!(keys.bases.keys().copied().collect::<Vec<_>>(), vec![epoch]);
        assert_eq!(keys.active_epoch, epoch);
        assert_eq!(
            restored.network_keys[&7].bases,
            state.network_keys[&7].bases
        );
        assert_eq!(
            restored.identity.as_ref().unwrap().signer.public(),
            state.identity.as_ref().unwrap().signer.public()
        );
    }

    /// Builds the epochs of a network, which are recorded on a gateway.
    fn network_keys(active_epoch: u64, epochs: &[u64]) -> NetworkKeys {
        NetworkKeys {
            active_epoch,
            bases: epochs.iter().map(|epoch| (*epoch, vec![0u8; 32])).collect(),
            replaced: None,
        }
    }

    #[test]
    fn only_a_recorded_epoch_can_be_activated() {
        let mut state = MlsState::new_in_memory();
        state.network_keys.insert(5, network_keys(1, &[1, 2]));

        // an epoch, which the gateway didn't follow yet, would leave it without the key
        assert!(matches!(
            state.activate_epoch(5, 3),
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(state.activate_epoch(5, 2).is_ok());
        assert_eq!(state.network_keys[&5].active_epoch, 2);
        // activating the active epoch again changes nothing, an older one is refused
        assert!(state.activate_epoch(5, 2).is_ok());
        assert!(matches!(
            state.activate_epoch(5, 1),
            Err(ErrorResponse::Conflict(_))
        ));
        assert!(matches!(
            state.activate_epoch(6, 1),
            Err(ErrorResponse::NotFound(_))
        ));
    }

    #[test]
    fn only_the_epochs_before_the_active_one_can_be_retired() {
        let mut state = MlsState::new_in_memory();
        state.network_keys.insert(5, network_keys(2, &[1, 2, 3]));

        // epoch 2 is still used for the outgoing traffic
        assert!(matches!(
            state.retire_epochs(5, 3),
            Err(ErrorResponse::Conflict(_))
        ));
        assert!(state.retire_epochs(5, 2).is_ok());
        assert_eq!(
            state.network_keys[&5]
                .bases
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![2, 3]
        );
        // retiring again changes nothing
        assert!(state.retire_epochs(5, 2).is_ok());
        assert_eq!(state.network_keys[&5].bases.len(), 2);
    }

    #[test]
    fn a_new_epoch_is_recorded_without_being_activated() {
        let mut state = MlsState::new_in_memory();
        state.create_identity("10.0.0.5".to_string()).unwrap();
        crate::core::mls::group::create_group(&mut state, 7).unwrap();
        let first = state.reset_epochs(7).unwrap();

        // the group moves into the next epoch, for example by a self-update
        let identity = state.identity.as_ref().unwrap();
        let group = state.groups.get_mut(&7).unwrap();
        group
            .self_update(
                &state.provider,
                &identity.signer,
                openmls::prelude::LeafNodeParameters::default(),
            )
            .unwrap();
        group.merge_pending_commit(&state.provider).unwrap();
        let second = state.record_epoch(7).unwrap();

        assert_eq!(second, first + 1);
        let keys = &state.network_keys[&7];
        assert_eq!(keys.active_epoch, first);
        assert_eq!(
            keys.bases.keys().copied().collect::<Vec<_>>(),
            vec![first, second]
        );
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
