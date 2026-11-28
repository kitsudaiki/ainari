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

//! Subscriptions of the gateway to the groups of its networks.
//!
//! The gateway subscribes to the group of every network, which has a VM behind it and an
//! encrypted route towards another host, and unsubscribes from the groups, which it doesn't need
//! anymore. The izakaya tells with every subscription, if the gateway has to create the group or
//! waits for its welcome.

use std::collections::BTreeSet;

use crate::config::{CONFIG, INTERNAL_API_KEY};
use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::mls_key_exchange::gateway::apply_keys;
use crate::core::mls_key_exchange::group;
use crate::core::mls_key_exchange::state::MLS_STATE_HANDLE;

use ainari_api_structs::mls_structs::*;
use ainari_clients::izakaya as izakaya_clients;
use ainari_common::config::Endpoint;
use ainari_common::error::AinariError;

/// Subscribes the gateway to the group of a network and follows the answer of the izakaya.
///
/// # Arguments
/// * `izakaya` - Endpoint of the izakaya
/// * `client_id` - Identity of this gateway
/// * `vni` - Tenant of the network
/// * `has_group` - The gateway holds the group already
pub async fn subscribe(izakaya: &Endpoint, client_id: &str, vni: u32, has_group: bool) {
    let result = izakaya_clients::subscribe_mls_group(
        izakaya,
        &INTERNAL_API_KEY,
        vni,
        client_id,
        has_group,
        CONFIG.skip_tls_verification,
    )
    .await;
    match result {
        Ok(resp) => {
            if let Err(e) = handle_subscription(vni, resp.action, has_group).await {
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

/// Unsubscribes the gateway from the group of a network and leaves the group.
///
/// # Arguments
/// * `izakaya` - Endpoint of the izakaya
/// * `client_id` - Identity of this gateway
/// * `vni` - Tenant of the network
///
/// # Returns
/// `true` if the gateway left the group, `false` if the izakaya couldn't be told and the
/// unsubscription is tried again later
pub async fn unsubscribe(izakaya: &Endpoint, client_id: &str, vni: u32) -> bool {
    let result = izakaya_clients::unsubscribe_mls_group(
        izakaya,
        &INTERNAL_API_KEY,
        vni,
        client_id,
        CONFIG.skip_tls_verification,
    )
    .await;
    match result {
        Ok(()) | Err(AinariError::NotFound(_)) => {
            leave_group(vni).await;
            true
        }
        Err(e) => {
            log::warn!("Failed to unsubscribe from the MLS-group of tenant {vni}: {e}");
            false
        }
    }
}

/// Lists the networks, which need encryption on this gateway: a VM of the network is behind this
/// gateway and the network is reached over an encrypted route towards another host.
pub async fn encrypted_networks() -> BTreeSet<u32> {
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
