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

//! Delivery of the messages of the gateway over the izakaya.

use crate::config::{CONFIG, INTERNAL_API_KEY};
use crate::core::mls_key_exchange::commit;
use crate::core::mls_key_exchange::messages::OutgoingMessage;
use crate::core::mls_key_exchange::pair_messages;
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
        commit::clear_pending(mls, vni);
        return Err(e);
    }
    let change = commit::pending_change(mls, vni);
    commit::merge_pending(mls, vni).map_err(|e| format!("{e:?}"))?;
    mls.apply_commit(vni, &change)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

/// Hands the queued messages of the pairs to the izakaya. A message, which the izakaya didn't
/// accept, stays queued for the next call.
///
/// # Arguments
/// * `izakaya` - Endpoint of the izakaya
pub async fn send_pending_messages(izakaya: &Endpoint) {
    let mut mls = MLS_STATE_HANDLE.lock().await;
    if mls.pending_out.is_empty() {
        return;
    }
    let sender = mls.client_id().unwrap_or_default().to_string();
    let sent = pair_messages::flush(&mut mls, |message| {
        let sender = sender.clone();
        async move { deliver(izakaya, &sender, std::slice::from_ref(&message)).await }
    })
    .await;
    if sent && let Err(e) = mls.save() {
        log::error!("Failed to persist the MLS-state: {e:?}");
    }
}
