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

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::core::routing::{create_overlay_route, resolve_address, torii_of_host};
use crate::database::address_table;
use crate::database::address_table::AddressEntry;
use crate::database::host_table;
use crate::database::host_table::{HostEntry, HostResources};
use crate::database::meta_virtual_machine_table;
use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::RouteReq;
use ainari_api_structs::user_context::UserContext;
use ainari_api_structs::virtual_machine_structs::*;
use ainari_clients::endpoints::*;
use ainari_clients::network_interface as network_interface_clients;
use ainari_clients::proxy as proxy_clients;
use ainari_clients::quota::get_quota;
use ainari_clients::route as route_clients;
use ainari_clients::virtual_machine as virtual_machine_clients;
use ainari_common::config::Endpoints;
use ainari_common::enums::DbError;

/// Reserves a new virtual_machine on one of the sakura-hosts
///
/// The virtual_machine is only reserved here: it gets a host, an address with a TAP-device on the
/// gateways and a proxy-port, but no image and no public-key yet. It is created and booted by the
/// create-endpoint of the sakura-host afterwards.
///
/// # Arguments
/// * `body` - Values of the new virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(CreatedJson<VirtualMachineResp>)` with the reserved virtual_machine on success
/// * `Err(ErrorResponse)` with an appropriate error on failure
#[api_operation(
    tag = "virtual_machine",
    summary = "Reserve new virtual_machine",
    description = r###"Reserve a new virtual_machine together with its address within the given network.

The virtual_machine is placed on a random sakura-host, which has enough free cores, memory and
disk-space for it. The image and the public-key are not set here, but by the following
create-call."###,
    error_code = 400,
    error_code = 401,
    error_code = 409,
    error_code = 500
)]
pub async fn reserve_virtual_machine(
    body: Json<VirtualMachineCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<VirtualMachineResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    // the sakura-host expects the memory in bytes
    let memory_size_bytes = body.memory_size.checked_mul(1024 * 1024).ok_or_else(|| {
        ErrorResponse::BadRequest("Invalid input: memory_size is too large".to_string())
    })?;

    check_quota(&context).await?;

    // TODO: add check if network-, image- and public-key-uuid exist

    let (selected_host, allocated_resources) = select_host(
        body.number_of_cores,
        body.memory_size,
        body.disk_size,
        &context,
    )?;

    // parse uuid-string
    let sakura_uuid = convert_uuid(&selected_host.uuid)?;

    // give the allocated resources back to the host, if the virtual_machine can not be reserved
    let (virtual_machine_resp, proxy_uuid) =
        match prepare_selected_host(&selected_host, &body, memory_size_bytes, &context).await {
            Ok(result) => result,
            Err(e) => {
                if host_table::release_host_resources(&sakura_uuid, &allocated_resources).is_err() {
                    log::error!("Failed to release resources of host with UUID '{sakura_uuid}'.");
                }
                return Err(e);
            }
        };

    // add new virtual_machine to database
    let virtual_machine_uuid = virtual_machine_resp.uuid;
    meta_virtual_machine_table::add_new_meta_virtual_machine(
        &virtual_machine_uuid,
        &body.name,
        &sakura_uuid,
        &proxy_uuid,
        &allocated_resources,
        &context,
    )
    .map_err(|e| {
        log::error!(
            "Failed to add virtual_machine with UUID '{virtual_machine_uuid}' to database with error: {e}."
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(CreatedJson(virtual_machine_resp))
}

/// Selects the sakura-host, which runs the new virtual_machine, and allocates the resources of
/// the new virtual_machine on it
///
/// Only hosts, which have enough free cores, memory and disk-space for the new virtual_machine,
/// are considered and a random one of them is selected. The check and the allocation happen
/// atomically within the database, so parallel requests can not over-allocate a host.
///
/// # Arguments
/// * `number_of_cores` - Requested number of cores of the new virtual_machine
/// * `memory_size` - Requested memory of the new virtual_machine in MiB
/// * `disk_size` - Requested disk-size of the new virtual_machine in GiB
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok((HostEntry, HostResources))` with the selected host and the resources allocated on it
/// * `Err(ErrorResponse)` if there is no host with enough free resources or the hosts can not
///   be read from the database
fn select_host(
    number_of_cores: i32,
    memory_size: i64,
    disk_size: i64,
    context: &UserContext,
) -> Result<(HostEntry, HostResources), ErrorResponse> {
    let requested = HostResources {
        number_of_cores: i64::from(number_of_cores),
        memory_size,
        disk_space: disk_size,
    };

    match host_table::allocate_host_resources(&requested, context) {
        Ok(host) => Ok((host, requested)),
        Err(DbError::NotFound) => {
            log::error!(
                "No host with enough free resources for new virtual_machine: \
                 cores: {}, memory: {} MiB, disk: {} GiB.",
                requested.number_of_cores,
                requested.memory_size,
                requested.disk_space
            );
            Err(ErrorResponse::Conflict(
                "No host with enough free resources for the virtual_machine.".to_string(),
            ))
        }
        Err(DbError::InternalError) => {
            log::error!("Failed to select host for new virtual_machine from database.");
            Err(ErrorResponse::InternalError("Internal Error".to_string()))
        }
    }
}

/// Prepares everything, which the new virtual_machine requires on the selected sakura-host
///
/// This reserves the address of the virtual_machine, prepares its network on the gateways, creates
/// the entry of the virtual_machine on the sakura-host and makes it reachable by a proxy-port of
/// the torii.
///
/// # Arguments
/// * `selected_host` - Sakura-host, which runs the new virtual_machine
/// * `body` - Values of the new virtual_machine
/// * `memory_size_bytes` - Memory of the new virtual_machine in bytes, as expected by sakura
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok((VirtualMachineResp, Uuid))` with the new virtual_machine and the uuid of its proxy
/// * `Err(ErrorResponse)` with an appropriate error on failure
async fn prepare_selected_host(
    selected_host: &HostEntry,
    body: &Json<VirtualMachineCreateReq>,
    memory_size_bytes: i64,
    context: &UserContext,
) -> Result<(VirtualMachineResp, Uuid), ErrorResponse> {
    // get the network of the virtual_machine to reserve an address within its subnet
    let network_data = network_table::get_network(&body.network_uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("network", &body.network_uuid, e))?;

    // reserve internal address, MAC-address and TAP-device-name for the virtual_machine. The
    // address of the selected host is stored with them, so the routes towards this
    // virtual_machine can be created on the other hosts of its network later. The tenant of the
    // network is stored with them as well: it is what keeps two networks, which use the same
    // subnet, apart in the datapath of the torii.
    let vm_address = address_table::reserve_new_address(
        &network_data.uuid,
        &network_data.subnet,
        &selected_host.address,
        context,
    )
    .map_err(|e| map_db_register_error("mac", e))?;

    // get endpoints from miko
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    // prepare the network of the virtual_machine, so sakura can attach the virtual_machine to the
    // TAP-device, when it creates the virtual_machine later
    prepare_network(&endpoints, selected_host, &vm_address, context).await?;

    // send request to the selected sakura-host to create a virtual_machine
    let mut virtual_machine_resp = virtual_machine_clients::create_virtual_machine(
        &selected_host.address,
        &context.token,
        &config::INTERNAL_API_KEY,
        &body.name,
        &body.network_uuid,
        body.number_of_cores,
        memory_size_bytes,
        body.disk_size,
        &vm_address.internal_ip,
        &vm_address.tap_name,
        &vm_address.mac_address,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // the UUID of the virtual_machine is only known now, so the address is linked with it
    // afterwards. The link is what a floating ip-address is attached by later.
    address_table::set_virtual_machine_of_address(&vm_address.uuid, &virtual_machine_resp.uuid)
        .map_err(|_| {
            log::error!(
                "Failed to link address '{}' with virtual_machine '{}' in database.",
                vm_address.uuid,
                virtual_machine_resp.uuid
            );
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    // send request to torii to create a proxy
    let proxy_resp = proxy_clients::create_proxy(
        &endpoints.torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &virtual_machine_resp.uuid,
        &selected_host.address,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // the proxy-port is the port, where the virtual_machine and its sakura-host are reachable
    virtual_machine_resp.torii_port = proxy_resp.port;

    // sakura provides the memory in bytes, but the api of hanami uses MiB
    virtual_machine_resp.memory_size /= 1024 * 1024;

    Ok((virtual_machine_resp, proxy_resp.uuid))
}

/// Prepares the network of a new virtual_machine on the gateways
///
/// The TAP-device of the virtual_machine and the route towards it are created on the torii of the
/// sakura-host, which runs the virtual_machine later. Both carry the tenant of the network, so the
/// torii can tell this virtual_machine apart from one of another network with the very same
/// address. In a setup with more than one torii, the
/// torii, which is reachable from the outside, additionally gets a route towards the sakura-host,
/// so the virtual_machine can be reached from outside of its host, and the virtual_machines of
/// the network are connected with each other. The floating ip-address of the virtual_machine is
/// not handled here, but by the floating_ip-endpoints.
///
/// # Arguments
/// * `endpoints` - Endpoints of the components, which contains the torii reachable from outside
/// * `selected_host` - Sakura-host, which runs the virtual_machine and its torii
/// * `vm_address` - Reserved address of the virtual_machine with its TAP-device and MAC-address
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the TAP-device and all routes are created
/// * `Err(ErrorResponse)` with an appropriate error on failure
async fn prepare_network(
    endpoints: &Endpoints,
    selected_host: &HostEntry,
    vm_address: &AddressEntry,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let host_torii = torii_of_host(&endpoints.torii, &selected_host.address)?;

    // register the TAP-device, which connects the virtual_machine to the datapath of its torii
    network_interface_clients::register_tap(
        &host_torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &vm_address.tap_name,
        vm_address.vni,
        Some(vm_address.mac_address.clone()),
        Some(vm_address.internal_ip),
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // route the traffic for the virtual_machine on its host onto the new TAP-device
    route_clients::create_route(
        &host_torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &RouteReq {
            dest_ip: vm_address.internal_ip,
            target_iface: vm_address.tap_name.clone(),
            vni: vm_address.vni,
            gateway_ip: None,
            next_hop_ip: None,
            next_hop_mac: None,
            encrypted: false,
        },
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // the addresses are compared and sent as ip-addresses, so a host, which is addressed by a
    // dns-name, is resolved here
    let host_ip = resolve_address(&selected_host.address).await?;
    let external_torii_ip = resolve_address(&endpoints.torii.internal_address).await?;

    // in a single-torii setup the torii of the host is the torii, which is reachable from the
    // outside, so the traffic already ends on the TAP-device and no second route is required
    if host_ip != external_torii_ip {
        // route the traffic for the virtual_machine from the outside to the torii of its host.
        // The target-interface is left empty, so the torii uses the interface of its own
        // underlay.
        create_overlay_route(
            &endpoints.torii,
            vm_address.internal_ip,
            host_ip,
            vm_address.vni,
            context,
        )
        .await?;
    }

    // the virtual_machines of a network reach each other directly, without a detour over the
    // torii at the edge of the network
    connect_to_virtual_machines_of_network(endpoints, host_ip, vm_address, context).await?;

    Ok(())
}

/// Connects a new virtual_machine with the virtual_machines, which already exist within the same
/// network
///
/// Every virtual_machine of a network has to reach every other one of that network, so the torii
/// of the new virtual_machine gets a route towards each of the other virtual_machines and the
/// torii of each of those virtual_machines gets a route towards the new one. Two virtual_machines
/// on the same host share the torii of that host, which already routes between their TAP-devices,
/// so they are skipped here.
///
/// The address of the sakura-host of a virtual_machine is stored together with its internal
/// address, so the torii, which owns the TAP-device of an older virtual_machine, can be addressed
/// here again.
///
/// # Arguments
/// * `endpoints` - Endpoints of the components, which contains the torii reachable from outside
/// * `new_host_ip` - Underlay-address of the sakura-host, which runs the new virtual_machine
/// * `vm_address` - Reserved address of the new virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the routes between all virtual_machines of the network are created
/// * `Err(ErrorResponse)` with an appropriate error on failure
async fn connect_to_virtual_machines_of_network(
    endpoints: &Endpoints,
    new_host_ip: Ipv4Addr,
    vm_address: &AddressEntry,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let addresses_of_network = address_table::list_addresses_of_network(&vm_address.network_uuid)
        .map_err(|e| {
        log::error!(
            "Failed to get the addresses of network '{}' from database: '{e}'",
            vm_address.network_uuid
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    let new_torii = torii_of_host(&endpoints.torii, &vm_address.host_address)?;

    for other_address in addresses_of_network {
        // the address of the new virtual_machine is part of the list too
        if other_address.uuid == vm_address.uuid {
            continue;
        }

        // entries of a database, which was created before the host-address was stored with the
        // addresses, can not be connected, because their torii is unknown
        if other_address.host_address.is_empty() {
            log::warn!(
                "Address '{}' has no host-address, so it is not connected to the new one.",
                other_address.uuid
            );
            continue;
        }

        let other_host_ip = resolve_address(&other_address.host_address).await?;

        // both virtual_machines run on the same host, whose torii already routes between their
        // TAP-devices
        if other_host_ip == new_host_ip {
            continue;
        }

        let other_torii = torii_of_host(&endpoints.torii, &other_address.host_address)?;

        // from the new virtual_machine to the older one ...
        create_overlay_route(
            &new_torii,
            other_address.internal_ip,
            other_host_ip,
            other_address.vni,
            context,
        )
        .await?;

        // ... and from the older virtual_machine back to the new one
        create_overlay_route(
            &other_torii,
            vm_address.internal_ip,
            new_host_ip,
            vm_address.vni,
            context,
        )
        .await?;
    }

    Ok(())
}

/// Asynchronously checks if the user's current number of meta_virtual_machines is within their quota limit.
///
/// This function performs two main operations:
/// 1. Counts the current number of meta_virtual_machines for the given user
/// 2. Retrieves the user's quota from the Miko endpoint and verifies if the quota is exceeded
///
/// # Arguments
///
/// * `context` - A reference to the UserContext containing authentication and user information
///
/// # Returns
///
/// * `Ok(())` - If the quota check passes (user is within their limit)
/// * `Err(ErrorResponse)` - If there's an error during the check or if the quota is exceeded
///
/// # Errors
///
/// This function will return an error in the following cases:
/// - Database error when counting meta_virtual_machines
/// - Network error when communicating with the Miko endpoint
/// - If the user has exceeded their meta_virtual_machine quota limit
async fn check_quota(context: &UserContext) -> Result<(), ErrorResponse> {
    // Get the current number of meta_virtual_machines for the user from the database
    // This count is used to compare against the user's quota limit
    let current_number_of_meta_virtual_machines =
        meta_virtual_machine_table::count_meta_virtual_machines(context).map_err(|e| {
            log::error!("Failed to count meta_virtual_machines in database.: {e}");
            ErrorResponse::InternalError("Internal Error".to_string())
        })?;

    // Retrieve the user's quota information from the Miko endpoint
    // The miko_endpoint is configured in the application settings
    let miko_endpoint = &config::CONFIG.miko;
    let quota = get_quota(
        miko_endpoint,
        &context.token,
        &context.user_id,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    // Convert the quota's maximum virtual_machine count to i64 for comparison
    let max_number_of_meta_virtual_machines = quota.max_virtual_machine as i64;

    // Check if the user has already exceeded their quota
    // If exceeded, return a Conflict error response
    if current_number_of_meta_virtual_machines as i64 >= max_number_of_meta_virtual_machines {
        return Err(ErrorResponse::Conflict(
            "Maximum number of meta_virtual_machines exceeded.".to_string(),
        ));
    }

    // If all checks pass, return Ok indicating the quota is not exceeded
    Ok(())
}
