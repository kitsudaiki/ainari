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

//! MLS-based key-exchange between the gateways of a network.
//!
//! Every gateway is a MLS-client and every network has its own MLS-group, which contains the
//! gateways of all hosts with a VM of that network, which talk to each other over encrypted
//! routes. The gateways run themselves (see `agent`): each one subscribes at the izakaya to the
//! group of every network, which it serves, and the izakaya coordinates the changes of the groups
//! and their key-rotations.
//!
//! Who may be member of which group, is decided by hanami alone: a gateway is only added with a
//! membership-grant, which hanami signed for exactly its identity and signature-key. The committer
//! checks the grant before it adds the gateway, and every other member checks it again, before it
//! follows the commit. So neither a compromised izakaya nor a compromised gateway can bring a
//! gateway into a group, which hanami didn't allow.
//!
//! The group never protects any traffic itself: each gateway derives the keys of its IPsec
//! connections from the epochs of the group (see `keys`), so the keys rotate with every change of
//! the group and a removed gateway can't follow anymore. A new epoch is only recorded with the
//! change of the group. The izakaya lets all members activate it and retire the older epochs one
//! phase after another, so both ends of a connection always have matching keys.
//!
//! Lock order: the MLS-state is always locked before the gateway-state.

pub mod agent;
pub mod group;
pub mod keys;
pub mod state;

use openmls::prelude::*;

use crate::config::{CONFIG, INTERNAL_API_KEY};
use crate::core::ebpf_interface::EBPF_INTERFACE_HANDLE;
use crate::core::ebpf_interface::EBPFInterface;
use crate::core::mls_key_exchange::group::OutgoingMessage;
use crate::core::mls_key_exchange::state::{MLS_STATE_HANDLE, MlsState};

use ainari_api_structs::mls_structs::MlsMessageReq;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::izakaya as izakaya_clients;
use ainari_common::config::Endpoint;

/// Asks miko for the address of the izakaya.
///
/// # Returns
/// The endpoint of the izakaya, or the reason, why it is not known
pub async fn izakaya_endpoint() -> Result<Endpoint, String> {
    let endpoints = get_endpoints(&CONFIG.miko, CONFIG.skip_tls_verification)
        .await
        .map_err(|e| format!("Failed to get the endpoints from miko: {e}"))?;

    if endpoints.izakaya.internal_address.is_empty() {
        return Err("Miko has no endpoint of the izakaya configured".to_string());
    }
    Ok(endpoints.izakaya)
}

/// Hands messages over to the izakaya for the delivery to the other gateways.
///
/// # Arguments
/// * `izakaya` - Endpoint of the izakaya
/// * `sender` - Identity of this gateway
/// * `messages` - The messages
///
/// # Returns
/// `Ok(())` once all messages are accepted by the izakaya
async fn deliver(
    izakaya: &Endpoint,
    sender: &str,
    messages: &[OutgoingMessage],
) -> Result<(), String> {
    for message in messages {
        let req = MlsMessageReq {
            group_id: message.group_id.clone(),
            epoch: message.epoch,
            message_type: message.message_type,
            sender: sender.to_string(),
            recipients: message.recipients.clone(),
            payload: message.payload.clone(),
            grants: message.grants.clone(),
        };
        izakaya_clients::send_mls_message(
            izakaya,
            &INTERNAL_API_KEY,
            &req,
            CONFIG.skip_tls_verification,
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to hand a {} to the izakaya: {e}",
                message.message_type
            )
        })?;
    }
    Ok(())
}

/// Delivers the staged change of a group and merges it afterwards, or drops it again, if it
/// couldn't be delivered. The new epoch is recorded, which installs its incoming keys, while the
/// outgoing traffic keeps using the active epoch.
///
/// # Arguments
/// * `mls` - The locked MLS-client
/// * `vni` - Tenant of the group
/// * `izakaya` - Endpoint of the izakaya
/// * `messages` - The messages of the change
///
/// # Returns
/// `Ok(())` once the group is in its new epoch
pub async fn deliver_and_merge(
    mls: &mut MlsState,
    vni: u32,
    izakaya: &Endpoint,
    messages: &[OutgoingMessage],
) -> Result<(), String> {
    let sender = mls.client_id().unwrap_or_default().to_string();
    if let Err(e) = deliver(izakaya, &sender, messages).await {
        group::clear_pending(mls, vni);
        return Err(e);
    }
    group::merge_pending(mls, vni).map_err(|e| format!("{e:?}"))?;
    mls.record_epoch(vni)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

/// Brings the keys of a network in line with its recorded epochs. Errors are logged, because the
/// change, which caused the call, already happened.
///
/// # Arguments
/// * `mls` - The locked MLS-client
/// * `ebpf_interf` - The locked gateway state
/// * `vni` - Tenant of the network
pub fn apply_keys(mls: &MlsState, ebpf_interf: &mut EBPFInterface, vni: u32) {
    let keys = mls.network_keys.get(&vni);
    if let Err(e) = keys::apply_network_keys(ebpf_interf, keys, mls.provider.crypto(), vni) {
        log::error!("Failed to apply the keys of tenant {vni}: {e}");
    }
}

/// Brings the keys of a network in line with its routes, after a route was added, changed or
/// removed.
///
/// # Arguments
/// * `vni` - Tenant of the network
pub async fn refresh_network_keys(vni: u32) {
    let mls = MLS_STATE_HANDLE.lock().await;
    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    apply_keys(&mls, &mut ebpf_interf, vni);
}

/// Installs the keys of all groups again after a restart of the gateway.
pub async fn restore_network_keys() {
    let mls = MLS_STATE_HANDLE.lock().await;
    let mut ebpf_interf = EBPF_INTERFACE_HANDLE.lock().await;
    for vni in mls.network_keys.keys() {
        apply_keys(&mls, &mut ebpf_interf, *vni);
    }
}
