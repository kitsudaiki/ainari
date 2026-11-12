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

use crate::config::{CONFIG, GRANT_PUBLIC_KEY, INTERNAL_API_KEY};
use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::mls_key_exchange::gateway::{
    apply_keys, drive_all_pairs, remove_idle_network_keys,
};
use crate::core::mls_key_exchange::izakaya::{izakaya_endpoint, send_pending_messages};
use crate::core::mls_key_exchange::messages::{self, Processed};
use crate::core::mls_key_exchange::state::MLS_STATE_HANDLE;
use crate::core::mls_key_exchange::{committer, group, pair_messages, subscriptions};
use crate::core::utils::get_local_ip;

use ainari_api_structs::mls_structs::*;
use ainari_clients::izakaya as izakaya_clients;
use ainari_common::config::Endpoint;

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
        let desired = subscriptions::encrypted_networks().await;
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
            if due {
                self.subscriptions.insert(*vni, Instant::now());
                subscriptions::subscribe(izakaya, client_id, *vni, held.contains(vni)).await;
            }
        }
        for vni in held.difference(&desired) {
            if subscriptions::unsubscribe(izakaya, client_id, *vni).await {
                self.subscriptions.remove(vni);
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
                        Ok(op) => {
                            committer::handle_operation(&self.grant_key, izakaya, client_id, op)
                                .await
                        }
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
        let processed = messages::process_message(
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
        pair_messages::retry_pending(&mut mls, &self.grant_key);

        mls.save().map_err(|e| format!("{e:?}"))?;
        let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
        apply_keys(&mls, &mut ebpf_interf, vni);
        Ok(())
    }

    /// Processes a message of the peer of a pair.
    async fn handle_application(&self, message: &MlsMessageResp) -> Result<(), String> {
        let mut mls = MLS_STATE_HANDLE.lock().await;
        if pair_messages::handle_message(&mut mls, message, &self.grant_key)? {
            mls.save().map_err(|e| format!("{e:?}"))?;
        }
        Ok(())
    }
}
