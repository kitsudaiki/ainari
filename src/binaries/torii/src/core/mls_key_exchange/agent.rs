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

//! Background-loop of the gateway, which takes part in the MLS-groups of its networks.
//!
//! Every second the agent
//!
//! 1. keeps enough key-packages at the izakaya, so other gateways can add this one,
//! 2. subscribes to the group of every network, which has a VM behind this gateway and an
//!    encrypted route towards another host, and unsubscribes from the groups of the networks,
//!    which don't need encryption on this gateway anymore,
//! 3. processes the messages, which the izakaya holds for it: welcomes and commits of the
//!    groups, changes of a group, which it has to make as committer, and the messages of the
//!    peers of its pairs,
//! 4. moves its pairs to the keys of new epochs, sends their messages and removes old keys,
//!    which no packet uses anymore.
//!
//! The decisions of the agent only depend on the routes of the gateway and on the grants of
//! hanami, so any number of hanami-instances can run without coordinating the groups.

use std::collections::{BTreeSet, HashMap};
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::config::{CONFIG, GRANT_PUBLIC_KEY, INTERNAL_API_KEY};
use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::mls_key_exchange::group::{self, OutgoingMessage, Processed};
use crate::core::mls_key_exchange::pairs;
use crate::core::mls_key_exchange::state::MLS_STATE_HANDLE;
use crate::core::mls_key_exchange::{
    apply_keys, deliver_and_merge, drive_all_pairs, izakaya_endpoint, remove_idle_network_keys,
    send_pending_messages,
};
use crate::core::utils::get_local_ip;

use ainari_api_structs::mls_structs::*;
use ainari_clients::izakaya as izakaya_clients;
use ainari_common::config::Endpoint;
use ainari_common::error::AinariError;

/// Time between two runs of the agent
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Time, after which a subscription is renewed. It tells the gateway, if the izakaya lost its
/// state, and lets a gateway, which waits for its welcome, ask again.
const RESUBSCRIBE_INTERVAL: Duration = Duration::from_secs(30);

/// Time between two checks of the key-packages, which are left at the izakaya
const KEY_PACKAGE_CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Number of key-packages, below which new ones are uploaded
const MIN_KEY_PACKAGES: u64 = 5;

/// Number of key-packages, which are uploaded at once
const KEY_PACKAGE_BATCH: u32 = 10;

/// Time, after which the endpoint of the izakaya is asked from miko again
const ENDPOINT_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// Starts the agent in the background, if the key-exchange is configured.
pub fn spawn_agent() {
    let Some(grant_key) = *GRANT_PUBLIC_KEY else {
        log::info!("The MLS key-exchange is not configured, so the gateway joins no group");
        return;
    };

    actix_web::rt::spawn(async move {
        let mut agent = Agent::new(grant_key);
        loop {
            actix_web::rt::time::sleep(POLL_INTERVAL).await;
            if let Err(e) = agent.run_once().await {
                log::warn!("MLS-agent: {e}");
            }
        }
    });
}

/// State of the agent, which is only kept in memory
struct Agent {
    /// Public key of hanami, which signs the grants
    grant_key: VerifyingKey,
    /// Endpoint of the izakaya together with the time, when it was asked from miko
    izakaya: Option<(Endpoint, Instant)>,
    /// Last subscription to the group of every network
    subscriptions: HashMap<u32, Instant>,
    /// Last check of the key-packages at the izakaya
    key_packages_checked: Option<Instant>,
}

impl Agent {
    fn new(grant_key: VerifyingKey) -> Self {
        Agent {
            grant_key,
            izakaya: None,
            subscriptions: HashMap::new(),
            key_packages_checked: None,
        }
    }

    /// Runs one loop of the agent.
    async fn run_once(&mut self) -> Result<(), String> {
        // without an underlay address the gateway has no identity yet
        if get_local_ip(&CONFIG.network.underlay_iface).is_none() {
            return Ok(());
        }
        // old keys are removed on their own, also while the izakaya is not reachable
        remove_idle_network_keys().await;

        let izakaya = self.izakaya().await?;
        let client_id = {
            let mut mls = MLS_STATE_HANDLE.lock().await;
            mls.ensure_identity()
                .map_err(|e| format!("no MLS-identity: {e:?}"))?
                .client_id
                .clone()
        };

        self.ensure_key_packages(&izakaya, &client_id).await?;
        self.reconcile_subscriptions(&izakaya, &client_id).await;
        let result = self.process_messages(&izakaya, &client_id).await;
        drive_all_pairs().await;
        send_pending_messages(&izakaya).await;
        result
    }

    /// Returns the endpoint of the izakaya, which is asked from miko from time to time.
    async fn izakaya(&mut self) -> Result<Endpoint, String> {
        if let Some((endpoint, fetched_at)) = &self.izakaya
            && fetched_at.elapsed() < ENDPOINT_REFRESH_INTERVAL
        {
            return Ok(Endpoint {
                public_address: endpoint.public_address.clone(),
                internal_address: endpoint.internal_address.clone(),
            });
        }

        let endpoint = izakaya_endpoint().await?;
        let copy = Endpoint {
            public_address: endpoint.public_address.clone(),
            internal_address: endpoint.internal_address.clone(),
        };
        self.izakaya = Some((endpoint, Instant::now()));
        Ok(copy)
    }

    /// Uploads new key-packages, if the izakaya has only a few left.
    async fn ensure_key_packages(
        &mut self,
        izakaya: &Endpoint,
        client_id: &str,
    ) -> Result<(), String> {
        if self
            .key_packages_checked
            .is_some_and(|checked| checked.elapsed() < KEY_PACKAGE_CHECK_INTERVAL)
        {
            return Ok(());
        }

        let count = izakaya_clients::get_key_package_count(
            izakaya,
            &INTERNAL_API_KEY,
            client_id,
            CONFIG.skip_tls_verification,
        )
        .await
        .map_err(|e| format!("Failed to count the key-packages: {e}"))?;

        if count.count < MIN_KEY_PACKAGES {
            let key_packages = {
                let mls = MLS_STATE_HANDLE.lock().await;
                let key_packages = group::create_key_packages(&mls, KEY_PACKAGE_BATCH)
                    .map_err(|e| format!("Failed to create key-packages: {e:?}"))?;
                // the private parts have to survive a restart, before a welcome uses them
                mls.save()
                    .map_err(|e| format!("Failed to persist the key-packages: {e:?}"))?;
                key_packages
            };
            izakaya_clients::upload_key_packages(
                izakaya,
                &INTERNAL_API_KEY,
                client_id,
                key_packages,
                CONFIG.skip_tls_verification,
            )
            .await
            .map_err(|e| format!("Failed to upload key-packages: {e}"))?;
            log::debug!("Uploaded {KEY_PACKAGE_BATCH} key-packages of '{client_id}'");
        }

        self.key_packages_checked = Some(Instant::now());
        Ok(())
    }

    /// Subscribes to the groups of the networks, which need encryption on this gateway, and
    /// unsubscribes from the others.
    async fn reconcile_subscriptions(&mut self, izakaya: &Endpoint, client_id: &str) {
        let desired = encrypted_networks().await;
        let held: BTreeSet<u32> = MLS_STATE_HANDLE
            .lock()
            .await
            .groups
            .keys()
            .copied()
            .collect();

        for vni in &desired {
            let due = self
                .subscriptions
                .get(vni)
                .is_none_or(|subscribed| subscribed.elapsed() >= RESUBSCRIBE_INTERVAL);
            if !due {
                continue;
            }
            self.subscriptions.insert(*vni, Instant::now());

            let has_group = held.contains(vni);
            match izakaya_clients::subscribe_mls_group(
                izakaya,
                &INTERNAL_API_KEY,
                *vni,
                client_id,
                has_group,
                CONFIG.skip_tls_verification,
            )
            .await
            {
                Ok(resp) => {
                    if let Err(e) = handle_subscription(*vni, resp.action, has_group).await {
                        log::error!("Failed to handle the subscription of tenant {vni}: {e}");
                    }
                }
                // hanami didn't grant the membership (yet)
                Err(AinariError::Forbidden(msg)) => {
                    log::debug!("Not allowed to join the MLS-group of tenant {vni}: {msg}");
                }
                Err(e) => {
                    log::warn!("Failed to subscribe to the MLS-group of tenant {vni}: {e}");
                }
            }
        }

        for vni in held.difference(&desired) {
            let result = izakaya_clients::unsubscribe_mls_group(
                izakaya,
                &INTERNAL_API_KEY,
                *vni,
                client_id,
                CONFIG.skip_tls_verification,
            )
            .await;
            match result {
                Ok(()) | Err(AinariError::NotFound(_)) => {
                    self.subscriptions.remove(vni);
                    leave_group(*vni).await;
                }
                Err(e) => {
                    log::warn!("Failed to unsubscribe from the MLS-group of tenant {vni}: {e}");
                }
            }
        }
    }

    /// Processes and acknowledges all messages, which wait for this gateway.
    async fn process_messages(
        &mut self,
        izakaya: &Endpoint,
        client_id: &str,
    ) -> Result<(), String> {
        let messages = izakaya_clients::list_mls_messages(
            izakaya,
            &INTERNAL_API_KEY,
            client_id,
            CONFIG.skip_tls_verification,
        )
        .await
        .map_err(|e| format!("Failed to fetch the MLS-messages: {e}"))?;

        for message in messages.messages {
            let result = match message.message_type {
                MlsMessageType::Welcome | MlsMessageType::Commit => {
                    self.handle_group_message(&message).await
                }
                MlsMessageType::Application => self.handle_application(&message).await,
                MlsMessageType::Operation => {
                    match serde_json::from_str::<MlsOperation>(&message.payload) {
                        Ok(op) => self.handle_operation(izakaya, client_id, op).await,
                        Err(e) => Err(format!("broken operation: {e}")),
                    }
                }
            };
            if let Err(e) = result {
                log::error!(
                    "Drop {} '{}' of group '{}' from '{}': {e}",
                    message.message_type,
                    message.uuid,
                    message.group_id,
                    message.sender
                );
            }

            // a message, which can't be processed, would block all following ones
            if let Err(e) = izakaya_clients::delete_mls_message(
                izakaya,
                &INTERNAL_API_KEY,
                client_id,
                &message.uuid,
                CONFIG.skip_tls_verification,
            )
            .await
            {
                log::error!("Failed to acknowledge mls-message '{}': {e}", message.uuid);
            }
        }
        Ok(())
    }

    /// Processes a welcome or a commit of another gateway.
    ///
    /// The new epoch is only recorded, which installs its incoming keys. The outgoing traffic of
    /// every pair switches, once its peer is ready (see `pairs`).
    async fn handle_group_message(&self, message: &MlsMessageResp) -> Result<(), String> {
        let mut mls = MLS_STATE_HANDLE.lock().await;
        let processed = group::process_message(
            &mut mls,
            message.message_type,
            &message.payload,
            &message.grants,
            &self.grant_key,
        )?;

        let vni = match processed {
            // a group, which replaces an older one of the network, keeps the keys of the older one
            // in use, until the pairs switched to the new one
            Processed::Joined {
                vni,
                replaced: true,
            } => {
                log::info!("Joined the new MLS-group of tenant {vni}, which replaces the old one");
                mls.replace_epochs(vni).map_err(|e| format!("{e:?}"))?;
                vni
            }
            // a joined group starts with its first epoch as the active one: the other members
            // can already receive with it, because they installed its keys with the commit
            Processed::Joined {
                vni,
                replaced: false,
            } => {
                log::info!("Joined the MLS-group of tenant {vni}");
                mls.reset_epochs(vni).map_err(|e| format!("{e:?}"))?;
                vni
            }
            Processed::Advanced { vni, change } => {
                mls.apply_commit(vni, &change)
                    .map_err(|e| format!("{e:?}"))?;
                vni
            }
            Processed::Left { vni } => {
                log::info!("Was removed from the MLS-group of tenant {vni}");
                mls.network_keys.remove(&vni);
                vni
            }
            Processed::Ignored => return Ok(()),
            Processed::Application { .. } | Processed::Pending => {
                return Err("message is no welcome or commit".to_string());
            }
        };
        // messages of the new epoch, which arrived before its commit, can be read now
        pairs::retry_pending(&mut mls, &self.grant_key);

        mls.save().map_err(|e| format!("{e:?}"))?;
        let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
        apply_keys(&mls, &mut ebpf_interf, vni);
        Ok(())
    }

    /// Processes a message of the peer of a pair.
    async fn handle_application(&self, message: &MlsMessageResp) -> Result<(), String> {
        let mut mls = MLS_STATE_HANDLE.lock().await;
        if pairs::handle_message(&mut mls, message, &self.grant_key)? {
            mls.save().map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }

    /// Makes a change of a group as its committer and reports the result to the izakaya.
    ///
    /// All additions and removals of the change go into one commit. A gateway, which can't be
    /// added, is left out and reported to the izakaya, the others are added anyway.
    async fn handle_operation(
        &self,
        izakaya: &Endpoint,
        client_id: &str,
        op: MlsOperation,
    ) -> Result<(), String> {
        let vni = op.vni;
        let description = describe(&op);
        let mut mls = MLS_STATE_HANDLE.lock().await;

        let mut refused = Vec::new();
        let result = if !mls.groups.contains_key(&vni) {
            Err(format!("this gateway has no MLS-group for tenant {vni}"))
        } else if op.update {
            match op.adds.is_empty() && op.removes.is_empty() {
                false => Err("a key-rotation can't be combined with other changes".to_string()),
                true => match group::stage_update(&mut mls, vni) {
                    Ok(commit) => {
                        let messages: Vec<OutgoingMessage> = commit.into_iter().collect();
                        deliver_and_merge(&mut mls, vni, izakaya, &messages).await
                    }
                    Err(e) => Err(format!("{e:?}")),
                },
            }
        } else {
            self.change_members(&mut mls, izakaya, &op, &mut refused)
                .await
        };

        if let Err(e) = mls.save() {
            log::error!("Failed to persist the MLS-state: {e:?}");
        }
        if result.is_ok() {
            let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
            apply_keys(&mls, &mut ebpf_interf, vni);
        }

        let done = MlsOperationDoneReq {
            op_uuid: op.op_uuid,
            client_id: client_id.to_string(),
            success: result.is_ok(),
            epoch: mls
                .groups
                .get(&vni)
                .map(|group| group.epoch().as_u64())
                .unwrap_or_default(),
            members: group::members(&mls, vni),
            reason: result.as_ref().err().cloned(),
            refused,
        };
        drop(mls);

        match &result {
            Ok(()) => log::info!(
                "Made the {description} for tenant {vni} in epoch {}",
                done.epoch
            ),
            Err(e) => log::warn!("Refused the {description} for tenant {vni}: {e}"),
        }
        for refusal in &done.refused {
            log::warn!(
                "Left '{}' out of the {description} for tenant {vni}: {}",
                refusal.client_id,
                refusal.reason
            );
        }

        izakaya_clients::finish_mls_operation(
            izakaya,
            &INTERNAL_API_KEY,
            vni,
            &done,
            CONFIG.skip_tls_verification,
        )
        .await
        .map_err(|e| format!("Failed to report the {description}: {e}"))
    }

    /// Adds and removes the gateways of an operation with one commit.
    ///
    /// A gateway, which is still member from an earlier membership, for example because it lost
    /// the group, loses its old leaf with the same commit, which adds its new key-package.
    ///
    /// # Arguments
    /// * `mls` - The locked MLS-client
    /// * `izakaya` - Endpoint of the izakaya
    /// * `op` - The operation
    /// * `refused` - Takes the gateways, which can't be added
    ///
    /// # Returns
    /// `Ok(())` once the group is in its new epoch, or the reason, why no commit was made
    async fn change_members(
        &self,
        mls: &mut crate::core::mls_key_exchange::state::MlsState,
        izakaya: &Endpoint,
        op: &MlsOperation,
        refused: &mut Vec<MlsOperationRefusal>,
    ) -> Result<(), String> {
        let vni = op.vni;
        let claim = |target: String, signature_key: String| async move {
            izakaya_clients::claim_key_package(
                izakaya,
                &INTERNAL_API_KEY,
                &target,
                Some(signature_key),
                CONFIG.skip_tls_verification,
            )
            .await
            .map(|claimed| claimed.key_package)
            .map_err(|e| format!("no key-package of '{target}': {e}"))
        };
        let (additions, left_out) = group::check_additions(
            mls,
            vni,
            &op.adds,
            &self.grant_key,
            Utc::now().timestamp(),
            claim,
        )
        .await;
        *refused = left_out;

        for addition in &additions {
            if group::is_member(mls, vni, &addition.client_id) {
                log::warn!(
                    "'{}' is still member of the MLS-group of tenant {vni}, re-add it",
                    addition.client_id
                );
            }
        }
        // a gateway, which already left the group, needs no removal
        let removals: Vec<String> = op
            .removes
            .iter()
            .filter(|target| group::is_member(mls, vni, target))
            .cloned()
            .collect();
        if additions.is_empty() && removals.is_empty() {
            return Err("nothing is left to change".to_string());
        }

        let messages =
            group::stage_change(mls, vni, additions, &removals).map_err(|e| format!("{e:?}"))?;
        deliver_and_merge(mls, vni, izakaya, &messages).await
    }
}

/// Describes an operation for the log.
///
/// # Arguments
/// * `op` - The operation
///
/// # Returns
/// For example "change (add: [10.0.0.2], remove: [])" or "key-rotation"
fn describe(op: &MlsOperation) -> String {
    if op.update {
        return "key-rotation".to_string();
    }
    let adds: Vec<&str> = op.adds.iter().map(|add| add.client_id.as_str()).collect();
    format!("change (add: {adds:?}, remove: {:?})", op.removes)
}

/// Lists the networks, which need encryption on this gateway: a VM of the network is behind this
/// gateway and the network is reached over an encrypted route towards another host.
async fn encrypted_networks() -> BTreeSet<u32> {
    let ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;

    let local: BTreeSet<u32> = ebpf_interf
        .routes
        .values()
        .filter(|route| route.gateway_ip.is_none())
        .filter(|route| {
            ebpf_interf
                .taps
                .get(&route.target_iface)
                .is_some_and(|tap| tap.vni == route.vni)
        })
        .map(|route| route.vni)
        .collect();
    let encrypted: BTreeSet<u32> = ebpf_interf
        .routes
        .values()
        .filter(|route| route.encrypted)
        .map(|route| route.vni)
        .collect();

    local.intersection(&encrypted).copied().collect()
}

/// Takes the result of a subscription over.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `action` - What the izakaya answered
/// * `has_group` - Whether the gateway holds the group
///
/// # Returns
/// `Ok(())` once the gateway did, what the izakaya asked for
async fn handle_subscription(
    vni: u32,
    action: MlsSubscribeAction,
    has_group: bool,
) -> Result<(), String> {
    match action {
        MlsSubscribeAction::Create => {
            let mut mls = MLS_STATE_HANDLE.lock().await;
            group::create_group(&mut mls, vni).map_err(|e| format!("{e:?}"))?;
            if has_group {
                // the old group is still in use by the other members, so its keys stay, until
                // the pairs switched to the new group
                log::warn!(
                    "The izakaya doesn't know the MLS-group of tenant {vni}, start a new one"
                );
                mls.replace_epochs(vni).map_err(|e| format!("{e:?}"))?;
            } else {
                mls.reset_epochs(vni).map_err(|e| format!("{e:?}"))?;
            }
            mls.save().map_err(|e| format!("{e:?}"))?;
            log::info!("Created the MLS-group of tenant {vni}");

            let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
            apply_keys(&mls, &mut ebpf_interf, vni);
        }
        MlsSubscribeAction::Joining => {
            log::debug!("Waiting for the welcome into the MLS-group of tenant {vni}");
        }
        MlsSubscribeAction::Member => {}
    }
    Ok(())
}

/// Drops the group of a network, which this gateway doesn't serve anymore, together with all
/// keys derived from it.
///
/// # Arguments
/// * `vni` - Tenant of the network
async fn leave_group(vni: u32) {
    let mut mls = MLS_STATE_HANDLE.lock().await;
    if let Err(e) = group::delete_group(&mut mls, vni) {
        log::warn!("Failed to delete the MLS-group of tenant {vni}: {e:?}");
    }
    mls.network_keys.remove(&vni);
    if let Err(e) = mls.save() {
        log::error!("Failed to persist the MLS-state: {e:?}");
    }
    log::info!("Left the MLS-group of tenant {vni}");

    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    apply_keys(&mls, &mut ebpf_interf, vni);
}
