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

use std::net::Ipv4Addr;

use actix_web::web::Path;
use apistos::actix::NoContent;
use apistos::api_operation;
use uuid::Uuid;

use crate::config;
use crate::core::routing::{delete_routes_to, torii_of_host};
use crate::database::address_table;
use crate::database::host_table;
use crate::database::meta_virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::proxy as proxy_clients;
use ainari_clients::virtual_machine as virtual_machine_clients;
use ainari_common::config::Endpoints;

#[api_operation(
    tag = "virtual_machine",
    summary = "Delete virtual_machine",
    description = r###"Delete a virtual_machine.

It is deleted on its sakura-host, its metadata is removed from the database, the
proxy, which is connected to it, is deleted on the torii and the routes from and
to the virtual_machine are removed from the gateways of its network."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 500
)]
pub async fn delete_virtual_machine(
    virtual_machine_uuid: Path<Uuid>,
    context: UserContext,
) -> Result<NoContent, ErrorResponse> {
    let virtual_machine_data =
        meta_virtual_machine_table::get_meta_virtual_machine(&virtual_machine_uuid, &context)
            .map_err(|e| {
                map_db_uuid_get_delete_error("virtual_machine-meta", &virtual_machine_uuid, e)
            })?;

    let sakura_uuid = convert_uuid(&virtual_machine_data.sakura_host_uuid)?;
    let proxy_uuid = convert_uuid(&virtual_machine_data.proxy_uuid)?;

    let host_data = host_table::get_host(&sakura_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("sakura-host", &sakura_uuid, e))?;

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // read the virtual_machine from its host, before it is deleted there, because its network
    // and its internal address are the only way to find the entry of the address-table, which
    // belongs to it
    let virtual_machine = virtual_machine_clients::get_virtual_machine(
        &host_data.address,
        &context.token,
        &config::INTERNAL_API_KEY,
        &virtual_machine_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // send request to sakura to delete the virtual_machine
    virtual_machine_clients::delete_virtual_machine(
        &host_data.address,
        &context.token,
        &config::INTERNAL_API_KEY,
        &virtual_machine_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // delete virtual_machine from database of hanami
    meta_virtual_machine_table::delete_meta_virtual_machine(&virtual_machine_uuid, &context)
        .map_err(|e| {
            map_db_uuid_get_delete_error("virtual_machine-meta", &virtual_machine_uuid, e)
        })?;

    // send request to torii to delete the proxy, which is connected to the virtual_machine
    proxy_clients::delete_proxy(
        &endpoints.torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &proxy_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // remove the routes from and to the virtual_machine and release its address
    cleanup_network(
        &endpoints,
        &virtual_machine.network_uuid,
        virtual_machine.internal_ip,
        &context,
    )
    .await?;

    Ok(NoContent)
}

/// Removes the address of a deleted virtual_machine from the gateways and from the database
///
/// The address of a virtual_machine is known by more than one gateway: the torii of its own host
/// routes it onto the TAP-device, the torii at the edge of the network routes it towards that
/// host and the torii of every other host with a virtual_machine of the same network routes it
/// there too. All of these routes are deleted here.
///
/// The routes in the other direction, from the deleted virtual_machine towards the other
/// virtual_machines of its network, belong to the torii of its host. They are only deleted, if
/// the deleted virtual_machine was the last one of that network on this host, because the other
/// virtual_machines of the host still use them.
///
/// The entry of the address-table is marked as deleted at the end, so the address, the
/// MAC-address and the name of the TAP-device can be used again.
///
/// # Arguments
/// * `endpoints` - Endpoints of the components, which contains the torii reachable from outside
/// * `network_uuid` - UUID of the network of the deleted virtual_machine
/// * `internal_ip` - Internal address of the deleted virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if all routes are deleted and the address is released
/// * `Err(ErrorResponse)` with an appropriate error on failure
async fn cleanup_network(
    endpoints: &Endpoints,
    network_uuid: &Uuid,
    internal_ip: Ipv4Addr,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    // an address, which is not in the database, was already released by an earlier call, so
    // there is nothing left to clean up
    let address = match address_table::get_address_by_internal_ip(network_uuid, &internal_ip) {
        Ok(address) => address,
        Err(_) => {
            log::warn!(
                "No address-entry for '{internal_ip}' in network '{network_uuid}', so no routes \
                 are deleted."
            );
            return Ok(());
        }
    };

    // the entry is marked as deleted first, so the addresses of the network, which are read
    // afterwards, are the virtual_machines, which keep their routes
    address_table::delete_address(&address.uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("address", &address.uuid, e))?;

    // an entry of a database, which was created before the host-address was stored with the
    // addresses, has no torii, which could be asked to delete the routes of its host
    if address.host_address.is_empty() {
        log::warn!(
            "Address '{}' has no host-address, so only the routes of the torii at the edge of \
             the network are deleted.",
            address.uuid
        );
        return delete_routes_to(&endpoints.torii, internal_ip, context).await;
    }

    let remaining_addresses =
        address_table::list_addresses_of_network(network_uuid).map_err(|e| {
            log::error!(
                "Failed to get the addresses of network '{network_uuid}' from database: '{e}'"
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    // the addresses of the hosts, whose torii knows the address of the deleted virtual_machine:
    // its own host and every other host, which runs a virtual_machine of the same network
    let mut host_addresses = vec![address.host_address.clone()];
    for other_address in &remaining_addresses {
        if !other_address.host_address.is_empty() {
            host_addresses.push(other_address.host_address.clone());
        }
    }
    host_addresses.sort();
    host_addresses.dedup();

    // the torii at the edge of the network routes the address of the virtual_machine towards its
    // host, so it has to forget it too
    delete_routes_to(&endpoints.torii, internal_ip, context).await?;

    for host_address in &host_addresses {
        let torii = torii_of_host(&endpoints.torii, host_address)?;

        // in a single-torii setup the torii of a host is the torii at the edge of the network,
        // whose routes were already deleted
        if torii.internal_address == endpoints.torii.internal_address {
            continue;
        }

        delete_routes_to(&torii, internal_ip, context).await?;
    }

    // the torii of the host keeps its routes towards the other virtual_machines of the network,
    // as long as it runs one of them itself
    let host_runs_other_virtual_machines = remaining_addresses
        .iter()
        .any(|other_address| other_address.host_address == address.host_address);
    if host_runs_other_virtual_machines {
        return Ok(());
    }

    let host_torii = torii_of_host(&endpoints.torii, &address.host_address)?;

    // in a single-torii setup the torii of the host is the torii at the edge of the network,
    // whose routes towards the other virtual_machines are the way to reach them from outside
    if host_torii.internal_address == endpoints.torii.internal_address {
        return Ok(());
    }

    // the routes, which led from the deleted virtual_machine to the other virtual_machines of
    // its network, are not used by anything on this host any more
    for other_address in &remaining_addresses {
        delete_routes_to(&host_torii, other_address.internal_ip, context).await?;
    }

    Ok(())
}
