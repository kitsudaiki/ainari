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

//! Membership of the gateways in the MLS-groups of the networks.
//!
//! Every network has a MLS-group, which contains the gateways of all hosts with a virtual_machine
//! of that network, which talk to each other over encrypted routes. The gateways derive the
//! IPsec-keys of these routes from the group, so hanami never sees a key.
//!
//! The gateways and izakaya run the groups on their own: a gateway subscribes to the group of a
//! network, as soon as it serves the network, and izakaya coordinates the changes and the
//! key-rotations of the group. hanami only decides, who may be member: for every host, which gets
//! a virtual_machine of a network, it signs a membership-grant, which names the identity and the
//! MLS signature-key of the torii of the host, and it revokes the grant, when the last
//! virtual_machine of the network is gone from the host. The gateways check the grants
//! themselves, so neither a compromised izakaya nor a compromised torii can join a group, which
//! hanami didn't allow.
//!
//! The grants are idempotent and need no coordination, so any number of hanami-instances can run
//! next to each other.
//!
//! hanami refreshes all grants regularly from its own database (see `spawn_grant_refresher`), so
//! izakaya gets them back on its own, if it lost its database, and the grants of the hosts, which
//! keep their virtual_machines, never expire.
//!
//! The signature-key of the torii of a host is pinned with the first grant. A torii, which shows
//! up with another key later, gets no grant anymore, until its host is registered again.
//!
//! The gateway at the edge of the network is never part of a group, because the routes from and
//! to it are never encrypted.

use std::collections::BTreeSet;
use std::net::Ipv4Addr;
use std::thread;
use std::time::Duration;

use chrono::Utc;

use crate::config;
use crate::core::routing::resolve_address;
use crate::database::address_table;
use crate::database::host_table::{self, HostEntry};
use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::mls_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::izakaya as izakaya_clients;
use ainari_clients::network_crypto as crypto_clients;
use ainari_common::config::{Endpoint, Endpoints};

/// Decides, if the traffic of a network is encrypted at all.
///
/// # Arguments
/// * `disable_encryption` - The flag of the network, which was given with its creation
///
/// # Returns
/// `true` if the encryption is enabled in the config and not disabled for the network. Only then
/// the network gets encrypted routes and a MLS-group.
pub fn network_encrypted(disable_encryption: bool) -> bool {
    config::CONFIG.network.mls_encryption && !disable_encryption
}

/// Decides, if the traffic between two hosts of a network is encrypted.
///
/// # Arguments
/// * `network_encrypted` - Whether the network is encrypted at all (see `network_encrypted`)
/// * `host_ip` - Underlay-address of one host
/// * `other_host_ip` - Underlay-address of the other host
/// * `external_torii_ip` - Address of the gateway at the edge of the network
///
/// # Returns
/// `true` if the network is encrypted and none of the two hosts is the edge of the network
pub fn is_encrypted(
    network_encrypted: bool,
    host_ip: Ipv4Addr,
    other_host_ip: Ipv4Addr,
    external_torii_ip: Ipv4Addr,
) -> bool {
    network_encrypted
        && host_ip != other_host_ip
        && host_ip != external_torii_ip
        && other_host_ip != external_torii_ip
}

/// Allows the torii of a host to join the MLS-group of a network, before the encrypted routes of
/// a new virtual_machine on that host are created.
///
/// The identity of the torii has to be the underlay-address of its host, which the routes of the
/// other hosts point at. Its signature-key is pinned with the first grant, every following grant
/// has to name the same key.
///
/// # Arguments
/// * `endpoints` - Endpoints of the components, which contains the izakaya
/// * `host` - The host of the new virtual_machine
/// * `host_ip` - Underlay-address of the host
/// * `torii` - Endpoint of the torii of the host
/// * `vni` - Tenant of the network
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` once izakaya holds the grant
/// * `Err(ErrorResponse)` with `Conflict` if the torii doesn't match the host, otherwise an
///   appropriate error on failure
pub async fn grant_membership(
    endpoints: &Endpoints,
    host: &HostEntry,
    host_ip: Ipv4Addr,
    torii: &Endpoint,
    vni: u32,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let skip_tls = config::CONFIG.skip_tls_verification;

    let identity = crypto_clients::get_mls_identity(
        torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        skip_tls,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    if identity.client_id != host_ip.to_string() {
        log::error!(
            "The torii of host {host_ip} has the MLS-identity '{}' instead of its address",
            identity.client_id
        );
        return Err(ErrorResponse::Conflict(format!(
            "The torii of host {host_ip} has a wrong MLS-identity"
        )));
    }
    let signature_key = pinned_signature_key(host, &identity.signature_key, &identity.client_id)?;

    store_grant(endpoints, &add_payload(vni, host_ip, signature_key)).await?;

    log::info!("Host {host_ip} may join the MLS-group of tenant {vni}");
    Ok(())
}

/// Builds the content of a grant, which allows a torii to join the group of a network.
///
/// # Arguments
/// * `vni` - Tenant of the network
/// * `host_ip` - Underlay-address of the host, which is the identity of its torii
/// * `signature_key` - The pinned MLS signature-key of the torii
///
/// # Returns
/// The content of the grant
fn add_payload(vni: u32, host_ip: Ipv4Addr, signature_key: String) -> MlsGrantPayload {
    let now = Utc::now().timestamp();
    let validity = i64::try_from(config::CONFIG.network.mls_grant_validity).unwrap_or(i64::MAX);
    MlsGrantPayload {
        vni,
        client_id: host_ip.to_string(),
        signature_key,
        action: MlsGrantAction::Add,
        issued_at: now,
        expires_at: now.saturating_add(validity),
    }
}

/// Starts the background-thread, which refreshes all grants regularly.
///
/// Every instance of hanami runs it. The grants are idempotent, so the refreshes of several
/// instances don't get in each others way.
///
/// # Arguments
/// * `interval` - Seconds between two refreshes
pub fn spawn_grant_refresher(interval: u64) {
    thread::spawn(move || {
        // the http-client is bound to an actix-runtime, so the thread gets its own one
        let system = actix_rt::System::new();
        loop {
            thread::sleep(Duration::from_secs(interval.max(1)));
            match system.block_on(refresh_grants()) {
                Ok(count) => log::debug!("Refreshed {count} MLS membership-grant(s)"),
                Err(e) => log::error!("Failed to refresh the MLS membership-grants: {e}"),
            }
        }
    });
}

/// Grants the membership again to the torii of every host, which runs a virtual_machine of an
/// encrypted network.
///
/// Only hosts with a pinned signature-key are covered: a host gets its key pinned with its first
/// grant, when hanami places a virtual_machine on it. The torii isn't asked for its key here,
/// so a torii, which changed its key, gets no grant.
///
/// # Returns
/// The number of refreshed grants, or the reason, why the refresh failed
async fn refresh_grants() -> Result<usize, String> {
    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(|e| format!("Failed to get the endpoints from miko: {e}"))?;
    let external_torii_ip = resolve_address(&endpoints.torii.internal_address)
        .await
        .map_err(|e| format!("Failed to resolve the torii at the edge: {e:?}"))?;

    let addresses = address_table::list_addresses()
        .map_err(|e| format!("Failed to list the addresses: {e}"))?;
    // the networks with disabled encryption have no MLS-group, so their hosts get no grants
    let unencrypted: BTreeSet<_> = network_table::list_networks_without_encryption()
        .map_err(|e| format!("Failed to list the networks without encryption: {e}"))?
        .into_iter()
        .collect();
    let memberships: BTreeSet<(String, u32)> = addresses
        .into_iter()
        .filter(|address| !address.host_address.is_empty())
        .filter(|address| !unencrypted.contains(&address.network_uuid))
        .map(|address| (address.host_address, address.vni))
        .collect();

    let mut count = 0;
    for (host_address, vni) in memberships {
        let host = match host_table::get_host_by_address(&host_address, &system_context()) {
            Ok(host) => host,
            Err(_) => continue,
        };
        let signature_key = match host_table::get_mls_signature_key(&host.uuid) {
            Ok(Some(key)) => key,
            _ => continue,
        };
        let host_ip = match resolve_address(&host_address).await {
            Ok(ip) if ip != external_torii_ip => ip,
            _ => continue,
        };

        match store_grant(&endpoints, &add_payload(vni, host_ip, signature_key)).await {
            Ok(()) => count += 1,
            Err(e) => {
                log::warn!("Failed to refresh the grant of host {host_ip} for tenant {vni}: {e:?}")
            }
        }
    }
    Ok(count)
}

/// Context of the background-thread, which isn't bound to a user
fn system_context() -> UserContext {
    UserContext {
        token: String::new(),
        user_id: "hanami".to_string(),
        project_id: String::new(),
        is_admin: true.to_string(),
        project_role: ainari_common::enums::ProjectRole::Admin.to_string(),
    }
}

/// Takes the membership in the MLS-group of a network away from the torii of a host, after the
/// last virtual_machine of the network is gone from the host.
///
/// izakaya removes the torii from the group, which rotates the keys of the remaining members, so
/// the keys, which the torii knows, become worthless.
///
/// # Arguments
/// * `endpoints` - Endpoints of the components, which contains the izakaya
/// * `host_ip` - Underlay-address of the host
/// * `vni` - Tenant of the network
///
/// # Returns
/// * `Ok(())` once izakaya took the revocation
/// * `Err(ErrorResponse)` with an appropriate error on failure
pub async fn revoke_membership(
    endpoints: &Endpoints,
    host_ip: Ipv4Addr,
    vni: u32,
) -> Result<(), ErrorResponse> {
    let now = Utc::now().timestamp();
    let validity = i64::try_from(config::CONFIG.network.mls_grant_validity).unwrap_or(i64::MAX);
    let payload = MlsGrantPayload {
        vni,
        client_id: host_ip.to_string(),
        // a revocation covers the identity, whatever key it uses
        signature_key: String::new(),
        action: MlsGrantAction::Remove,
        issued_at: now,
        expires_at: now.saturating_add(validity),
    };
    store_grant(endpoints, &payload).await?;

    log::info!("Host {host_ip} has to leave the MLS-group of tenant {vni}");
    Ok(())
}

/// Signs a grant and hands it to the izakaya.
async fn store_grant(
    endpoints: &Endpoints,
    payload: &MlsGrantPayload,
) -> Result<(), ErrorResponse> {
    if endpoints.izakaya.internal_address.is_empty() {
        log::error!("Miko has no endpoint of the izakaya configured");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    let grant = MlsGrant::sign(payload, &config::MLS_GRANT_SIGNING_KEY);
    izakaya_clients::store_mls_grant(
        &endpoints.izakaya,
        &config::INTERNAL_API_KEY,
        &grant,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)
}

/// Pins the signature-key of the torii of a host with its first grant and checks it with every
/// following one.
///
/// # Arguments
/// * `host` - The host
/// * `signature_key` - The key, which the torii reports now
///
/// # Returns
/// The pinned key, or `Conflict` if the torii reports another key than the pinned one
fn pinned_signature_key(
    host: &HostEntry,
    signature_key: &str,
    client_id: &str,
) -> Result<String, ErrorResponse> {
    let db_error = |e| map_db_uuid_get_delete_error("sakura-host", &host.uuid, e);

    host_table::pin_mls_signature_key(&host.uuid, signature_key, client_id).map_err(db_error)?;
    let pinned = host_table::get_mls_signature_key(&host.uuid)
        .map_err(db_error)?
        .unwrap_or_default();

    if pinned != signature_key {
        log::error!(
            "The torii of host '{}' shows another MLS signature-key than the pinned one. It gets \
             no grant anymore, until the host registers itself again.",
            host.uuid
        );
        return Err(ErrorResponse::Conflict(format!(
            "The MLS-identity of the torii of host '{}' changed",
            host.uuid
        )));
    }
    Ok(pinned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Seed of the signing-key of the development-setups
    const DEVELOPMENT_SEED: &str = "Veuv2tBnfEjOJwFRdf6rDF9CymxxQwvHvAb3vey8Xoo=";

    fn read(path: &str) -> String {
        fs::read_to_string(format!("{}/../../../{path}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    #[test]
    fn the_development_keys_belong_together() {
        let key = parse_signing_key(DEVELOPMENT_SEED).unwrap();
        let public_key = encode_verifying_key(&key);

        // hanami of the local stack signs with the seed, izakaya and the torii check with the
        // public key
        assert!(read("docker-compose.yml").contains(DEVELOPMENT_SEED));
        for config in [
            "example_configs/ainari/izakaya.toml",
            "example_configs/ainari/torii.toml",
            "testing/local_stack/configs/izakaya.toml",
            "testing/local_stack/configs/torii_vmm.toml",
        ] {
            assert!(read(config).contains(&public_key), "{config}");
        }
    }

    #[test]
    fn the_edge_of_the_network_is_never_encrypted() {
        let edge = Ipv4Addr::new(10, 0, 0, 254);
        let a = Ipv4Addr::new(10, 0, 0, 1);
        let b = Ipv4Addr::new(10, 0, 0, 2);

        assert!(is_encrypted(true, a, b, edge));
        assert!(!is_encrypted(true, a, edge, edge));
        assert!(!is_encrypted(true, edge, b, edge));
        assert!(!is_encrypted(true, a, a, edge));
        // a network with disabled encryption is never encrypted
        assert!(!is_encrypted(false, a, b, edge));
    }
}
