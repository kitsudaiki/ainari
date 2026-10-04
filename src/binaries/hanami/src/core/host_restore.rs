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

//! Brings the network of a sakura-host up to date, which registered itself again after a restart.
//!
//! The torii in front of a sakura-host keeps its database on the node, so it restores its
//! TAP-devices, routes and packet-filters itself, and sakura boots its virtual_machines again (see
//! its `restart_after_host_restart`). What the host can't do itself, is done here: a recreated
//! pod comes back on a new address, which is also the MLS-identity of its torii. The torii at the
//! edge and the torii of the other hosts still route the virtual_machines of the host to its old
//! address, and the groups still contain its old identity. Every step only changes, what differs,
//! so a host, which kept its address, is left as it is.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use uuid::Uuid;

use crate::config;
use crate::core::mls::{grant_membership, is_encrypted, network_encrypted, revoke_membership};
use crate::core::routing::{resolve_address, torii_of_host};
use crate::database::address_table::{self, AddressEntry};
use crate::database::host_table;
use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::{RouteReq, RouteResp};
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::route as route_clients;
use ainari_common::config::{Endpoint, Endpoints};
use ainari_common::enums::ProjectRole;

/// Seconds between two attempts to reach the torii of a restarted host
const RETRY_INTERVAL: u64 = 5;

/// Number of attempts to reach the torii of a restarted host, before the restore is given up
const MAX_ATTEMPTS: u32 = 120;

/// Number of runs of a restore, which failed in some of its steps, for example because the torii
/// of another host was not reachable
const MAX_RUNS: u32 = 5;

/// Seconds between two runs of a restore
const RUN_INTERVAL: u64 = 30;

/// Hosts, which are restored right now, so a host is never restored twice at the same time
static RESTORING: Mutex<Option<HashSet<Uuid>>> = Mutex::new(None);

/// Starts the restore of a host, which registered itself again, in the background.
///
/// The registration of the host has to answer right away, because sakura waits for it, before it
/// starts. The torii of the host may still be starting as well, so the restore waits for it.
///
/// # Arguments
/// * `host_uuid` - UUID of the host
/// * `previous_client_id` - MLS-identity of the torii of the host before the restart, if any
pub fn spawn_restore(host_uuid: Uuid, previous_client_id: Option<String>) {
    {
        let mut restoring = RESTORING.lock().expect("mutex poisoned");
        if !restoring.get_or_insert_with(HashSet::new).insert(host_uuid) {
            log::info!("Host '{host_uuid}' is restored already");
            return;
        }
    }

    thread::spawn(move || {
        // the http-client is bound to an actix-runtime, so the thread gets its own one
        let system = actix_rt::System::new();
        // every step only changes, what differs, so a run, which failed in some steps, is simply
        // repeated
        for run in 1..=MAX_RUNS {
            match system.block_on(restore_host(host_uuid, previous_client_id.clone())) {
                Ok(()) => {
                    log::info!("Restored the network of host '{host_uuid}'");
                    break;
                }
                Err(e) if run < MAX_RUNS => {
                    log::warn!(
                        "Failed to restore the network of host '{host_uuid}' completely, try \
                         again in {RUN_INTERVAL} seconds: {e:?}"
                    );
                    thread::sleep(Duration::from_secs(RUN_INTERVAL));
                }
                Err(e) => {
                    log::error!("Failed to restore the network of host '{host_uuid}': {e:?}")
                }
            }
        }
        if let Some(restoring) = RESTORING.lock().expect("mutex poisoned").as_mut() {
            restoring.remove(&host_uuid);
        }
    });
}

/// Brings the routes towards the virtual_machines of a host and the MLS-identity of its torii up
/// to date with the address, which the host has now.
///
/// A step, which fails, doesn't stop the others, so as much as possible works again, for example
/// when the torii of another host is restarting itself right now. The first error is returned at
/// the end, so the restore is repeated.
///
/// # Arguments
/// * `host_uuid` - UUID of the host
/// * `previous_client_id` - MLS-identity of the torii of the host before the restart, if any
///
/// # Returns
/// `Ok(())` once the routes and the MLS-identity are up to date
async fn restore_host(
    host_uuid: Uuid,
    previous_client_id: Option<String>,
) -> Result<(), ErrorResponse> {
    let context = system_context();
    let host = host_table::get_host(&host_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("sakura-host", &host_uuid, e))?;

    let addresses: Vec<AddressEntry> = address_table::list_addresses()
        .map_err(|e| {
            log::error!("Failed to list the addresses: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?
        .into_iter()
        .filter(|address| address.host_address == host.address && is_in_use(address))
        .collect();
    if addresses.is_empty() {
        return Ok(());
    }

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;
    let host_torii = torii_of_host(&endpoints.torii, &host.address)?;
    wait_for_torii(&host_torii, &context).await?;

    // the address is resolved only now, because a new pod gets its address with its start
    let host_ip = resolve_address(&host.address).await?;
    let external_torii_ip = resolve_address(&endpoints.torii.internal_address).await?;
    log::info!(
        "Restore the network of {} virtual_machine(s) of host '{}' at {host_ip}",
        addresses.len(),
        host.name
    );

    let mut first_error: Option<ErrorResponse> = None;
    let mut note = |result: Result<(), ErrorResponse>| {
        if let Err(e) = result {
            log::warn!("A step of the restore of host '{host_uuid}' failed: {e:?}");
            first_error.get_or_insert(e);
        }
    };

    // the routes from the edge of the network to the virtual_machines of the host
    for address in &addresses {
        note(restore_edge_route(&endpoints, host_ip, external_torii_ip, address, &context).await);
    }

    // the membership in the MLS-groups of the encrypted networks, with the identity, which the
    // torii has now
    let networks: BTreeMap<u32, Uuid> = addresses
        .iter()
        .map(|address| (address.vni, address.network_uuid))
        .collect();
    let mut encrypted_networks = BTreeSet::new();
    for (vni, network_uuid) in &networks {
        let disabled = network_table::is_encryption_disabled(network_uuid).unwrap_or(false);
        if network_encrypted(disabled) && host_ip != external_torii_ip {
            encrypted_networks.insert(*network_uuid);
            note(grant_membership(&endpoints, &host, host_ip, &host_torii, *vni, &context).await);
        }
    }

    // a torii, which came back on another address, has a new identity. The old one never answers
    // again, so it leaves the groups right away instead of blocking their rounds.
    if let Some(previous) = previous_client_id
        .as_deref()
        .and_then(|previous| previous.parse::<Ipv4Addr>().ok())
        .filter(|previous| *previous != host_ip)
    {
        for (vni, network_uuid) in &networks {
            if encrypted_networks.contains(network_uuid) {
                note(revoke_membership(&endpoints, previous, *vni).await);
            }
        }
    }

    // the routes between the virtual_machines of the host and the ones of the same networks on
    // the other hosts
    for address in &addresses {
        let encrypted = encrypted_networks.contains(&address.network_uuid);
        note(
            restore_connections(
                &endpoints,
                &host_torii,
                host_ip,
                external_torii_ip,
                address,
                encrypted,
                &context,
            )
            .await,
        );
    }

    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Checks, if an address belongs to a virtual_machine.
///
/// An address, whose virtual_machine was never created, for example because its reservation
/// failed halfway, has nothing to restore. Its routes would only get in the way of the real ones,
/// for example in another tenant with the same addresses.
fn is_in_use(address: &AddressEntry) -> bool {
    address.virtual_machine_uuid.is_some()
}

/// Waits until the torii of a host answers.
async fn wait_for_torii(torii: &Endpoint, context: &UserContext) -> Result<(), ErrorResponse> {
    for attempt in 1..=MAX_ATTEMPTS {
        match list_routes(torii, context).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt == MAX_ATTEMPTS => return Err(e),
            Err(_) => {
                log::debug!(
                    "The torii '{}' doesn't answer yet, attempt {attempt}",
                    torii.internal_address
                );
                actix_rt::time::sleep(Duration::from_secs(RETRY_INTERVAL)).await;
            }
        }
    }
    Ok(())
}

/// Changes the route from the edge of the network to a virtual_machine of the host to the address,
/// which the host has now.
async fn restore_edge_route(
    endpoints: &Endpoints,
    host_ip: Ipv4Addr,
    external_torii_ip: Ipv4Addr,
    address: &AddressEntry,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    if host_ip == external_torii_ip {
        return Ok(());
    }
    ensure_route(
        &endpoints.torii,
        overlay_route(address.internal_ip, host_ip, address.vni, false),
        context,
    )
    .await
}

/// Brings the routes between a virtual_machine of the restarted host and the virtual_machines of
/// its network on the other hosts up to date, in both directions. The routes on the other hosts
/// are changed to the address, which the restarted host has now. The routes of the host itself,
/// which its torii restored from its database, are only changed, if another host got a new address
/// as well, while the host was down. Every route is tried, also if another one failed.
async fn restore_connections(
    endpoints: &Endpoints,
    host_torii: &Endpoint,
    host_ip: Ipv4Addr,
    external_torii_ip: Ipv4Addr,
    address: &AddressEntry,
    encrypted: bool,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let others = address_table::list_addresses_of_network(&address.network_uuid).map_err(|e| {
        log::error!(
            "Failed to get the addresses of network '{}': {e}",
            address.network_uuid
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let mut first_error = None;
    for other in others {
        if other.uuid == address.uuid || other.host_address.is_empty() || !is_in_use(&other) {
            continue;
        }
        let other_host_ip = resolve_address(&other.host_address).await?;
        // the torii of the host routes between its own TAP-devices
        if other_host_ip == host_ip {
            continue;
        }
        let other_torii = torii_of_host(&endpoints.torii, &other.host_address)?;
        let route_encrypted = is_encrypted(encrypted, host_ip, other_host_ip, external_torii_ip);

        let towards_other = ensure_route(
            host_torii,
            overlay_route(other.internal_ip, other_host_ip, other.vni, route_encrypted),
            context,
        )
        .await;
        let back = ensure_route(
            &other_torii,
            overlay_route(address.internal_ip, host_ip, address.vni, route_encrypted),
            context,
        )
        .await;
        if let Err(e) = towards_other.and(back) {
            first_error.get_or_insert(e);
        }
    }
    match first_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Builds the request of a route towards a virtual_machine on another host. The target-interface
/// is left empty, so the torii uses the interface of its own underlay.
fn overlay_route(dest_ip: Ipv4Addr, gateway_ip: Ipv4Addr, vni: u32, encrypted: bool) -> RouteReq {
    RouteReq {
        dest_ip,
        target_iface: String::new(),
        vni,
        gateway_ip: Some(gateway_ip),
        next_hop_ip: None,
        next_hop_mac: None,
        encrypted,
    }
}

/// Lists the routes of a torii.
async fn list_routes(
    torii: &Endpoint,
    context: &UserContext,
) -> Result<Vec<RouteResp>, ErrorResponse> {
    Ok(route_clients::list_route(
        torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?
    .routes)
}

/// Checks, if an existing route towards another host already leads to the host and has the
/// encryption, which a request wants.
fn route_matches(route: &RouteResp, req: &RouteReq) -> bool {
    route.gateway_ip == req.gateway_ip && route.encrypted == req.encrypted
}

/// Makes sure, that a torii has exactly one route towards an address of a tenant, which leads,
/// where the request wants it to.
///
/// A matching route is kept, a route with another target is updated, so it keeps its UUID, and a
/// missing route is created. Further routes towards the same address are deleted.
///
/// # Arguments
/// * `torii` - The torii
/// * `req` - The route, which the torii has to have
/// * `context` - Context of the restore
///
/// # Returns
/// `Ok(())` once the torii has the route
async fn ensure_route(
    torii: &Endpoint,
    req: RouteReq,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let existing: Vec<RouteResp> = list_routes(torii, context)
        .await?
        .into_iter()
        .filter(|route| route.vni == req.vni && route.dest_ip == req.dest_ip)
        .collect();

    let keep = existing
        .iter()
        .position(|route| route_matches(route, &req))
        .or(if existing.is_empty() { None } else { Some(0) });

    match keep {
        Some(index) if route_matches(&existing[index], &req) => {}
        Some(index) => {
            log::info!(
                "Change the route towards {} of tenant {} on '{}'",
                req.dest_ip,
                req.vni,
                torii.internal_address
            );
            route_clients::update_route(
                torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                &existing[index].uuid,
                &req,
                config::CONFIG.skip_tls_verification,
            )
            .await
            .map_err(map_ainari_error_to_api_response)?;
        }
        None => {
            route_clients::create_route(
                torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                &req,
                config::CONFIG.skip_tls_verification,
            )
            .await
            .map_err(map_ainari_error_to_api_response)?;
        }
    }

    for (index, route) in existing.iter().enumerate() {
        if Some(index) != keep {
            route_clients::delete_route(
                torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                &route.uuid,
                config::CONFIG.skip_tls_verification,
            )
            .await
            .map_err(map_ainari_error_to_api_response)?;
        }
    }
    Ok(())
}

/// Context of the restore, which isn't bound to a user. The torii authorize the internal
/// endpoints by the internal API-key alone.
fn system_context() -> UserContext {
    UserContext {
        token: String::new(),
        user_id: "hanami".to_string(),
        project_id: String::new(),
        is_admin: true.to_string(),
        project_role: ProjectRole::Admin.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(gateway_ip: Option<Ipv4Addr>, target_iface: &str, encrypted: bool) -> RouteResp {
        RouteResp {
            uuid: Uuid::new_v4(),
            vni: 5,
            dest_ip: Ipv4Addr::new(192, 168, 100, 2),
            target_iface: target_iface.to_string(),
            gateway_ip,
            next_hop_ip: None,
            next_hop_mac: None,
            encrypted,
        }
    }

    #[test]
    fn a_route_to_the_old_address_of_a_host_doesnt_match() {
        let old = Ipv4Addr::new(10, 42, 8, 10);
        let new = Ipv4Addr::new(10, 42, 8, 11);
        let req = overlay_route(Ipv4Addr::new(192, 168, 100, 2), new, 5, true);

        assert!(route_matches(&route(Some(new), "eth0", true), &req));
        assert!(!route_matches(&route(Some(old), "eth0", true), &req));
        // the encryption is part of the route
        assert!(!route_matches(&route(Some(new), "eth0", false), &req));
    }
}
