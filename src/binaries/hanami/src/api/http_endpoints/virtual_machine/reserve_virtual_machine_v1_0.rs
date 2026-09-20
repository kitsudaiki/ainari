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

use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};

use actix_web::web::Json;
use apistos::actix::CreatedJson;
use apistos::api_operation;
use rand::prelude::IndexedRandom;
use uuid::Uuid;
use validator::Validate;

use crate::config;
use crate::database::address_table;
use crate::database::address_table::AddressEntry;
use crate::database::host_table;
use crate::database::host_table::HostEntry;
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
use ainari_common::config::{Endpoint, Endpoints};

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
    summary = "Create new virtual_machine",
    description = r###"Create new virtual_machine."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn reserve_virtual_machine(
    body: Json<VirtualMachineCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<VirtualMachineResp>, ErrorResponse> {
    // validate incoming json
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    check_quota(&context).await?;

    // TODO: add check if network-, image- and public-key-uuid exist

    let selected_host = select_host(&context)?;
    let (virtual_machine_resp, proxy_uuid) =
        prepare_selected_host(&selected_host, &body, &context).await?;

    // parse uuid-string
    let sakura_uuid = convert_uuid(&selected_host.uuid)?;

    // add new virtual_machine to database
    let virtual_machine_uuid = virtual_machine_resp.uuid;
    meta_virtual_machine_table::add_new_meta_virtual_machine(
        &virtual_machine_uuid,
        &body.name,
        &sakura_uuid,
        &proxy_uuid,
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

/// Selects the sakura-host, which runs the new virtual_machine
///
/// # Arguments
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(HostEntry)` with a randomly selected host on success
/// * `Err(ErrorResponse)` if there is no host or the hosts can not be read from the database
fn select_host(context: &UserContext) -> Result<HostEntry, ErrorResponse> {
    // list all avaialble hosts
    let hosts = host_table::list_hosts(context).map_err(|e| {
        log::error!("Failed to get list of hosts form database: '{e}'");
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    // check that there is at least one host
    if hosts.is_empty() {
        log::error!("No hosts to schedule new virtual_machine.");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    // select first host
    let mut rng = rand::rng();
    let selected_host = if let Some(host) = hosts.choose(&mut rng) {
        host
    } else {
        log::error!("No hosts with list-position 0 doesn't exist.");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    };

    Ok(selected_host.clone())
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
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok((VirtualMachineResp, Uuid))` with the new virtual_machine and the uuid of its proxy
/// * `Err(ErrorResponse)` with an appropriate error on failure
async fn prepare_selected_host(
    selected_host: &HostEntry,
    body: &Json<VirtualMachineCreateReq>,
    context: &UserContext,
) -> Result<(VirtualMachineResp, Uuid), ErrorResponse> {
    // get the network of the virtual_machine to reserve an address within its subnet
    let network_data = network_table::get_network(&body.network_uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("network", &body.network_uuid, e))?;

    // reserve internal address, MAC-address and TAP-device-name for the virtual_machine
    let vm_address =
        address_table::reserve_new_address(&network_data.uuid, &network_data.subnet, context)
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
        body.memory_size,
        &vm_address.internal_ip,
        &vm_address.tap_name,
        &vm_address.mac_address,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

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

    Ok((virtual_machine_resp, proxy_resp.uuid))
}

/// Prepares the network of a new virtual_machine on the gateways
///
/// The TAP-device of the virtual_machine and the route towards it are created on the torii of the
/// sakura-host, which runs the virtual_machine later. In a setup with more than one torii, the
/// torii, which is reachable from the outside, additionally gets a route towards the sakura-host,
/// so the virtual_machine can be reached from outside of its host. The floating ip-address of the
/// virtual_machine is not handled here, but by the floating_ip-endpoints.
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
    if host_ip == external_torii_ip {
        return Ok(());
    }

    // route the traffic for the virtual_machine from the outside to the torii of its host. The
    // target-interface is left empty, so the torii uses the interface of its own underlay.
    route_clients::create_route(
        &endpoints.torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &RouteReq {
            dest_ip: vm_address.internal_ip,
            target_iface: String::new(),
            gateway_ip: Some(host_ip),
            next_hop_ip: None,
            next_hop_mac: None,
            encrypted: false,
        },
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    Ok(())
}

/// Builds the endpoint of the torii, which runs on a sakura-host
///
/// The torii of a sakura-host listens on the address of that host and uses the same port as the
/// torii, which is reachable from the outside.
///
/// # Arguments
/// * `torii_endpoint` - Endpoint of the torii, which is reachable from the outside
/// * `host_address` - Address of the sakura-host
///
/// # Returns
/// * `Ok(Endpoint)` with the endpoint of the torii of the sakura-host
/// * `Err(ErrorResponse)` if one of the two addresses has no port
fn torii_of_host(torii_endpoint: &Endpoint, host_address: &str) -> Result<Endpoint, ErrorResponse> {
    let torii_address = &torii_endpoint.internal_address;
    let (scheme, rest) = split_scheme(torii_address);

    let port = match rest.rsplit_once(':') {
        Some((_, port)) => port,
        None => {
            log::error!("Torii-address '{torii_address}' has no port.");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(Endpoint {
        internal_address: format!("{scheme}{}:{port}", address_host(host_address)),
        public_address: torii_endpoint.public_address.clone(),
    })
}

/// Resolves the host of an address into an ip-address
///
/// The addresses of the sakura-hosts are ip-addresses already, which are returned as they are.
/// An address with a dns-name, like the configured address of a torii, is resolved by the
/// resolver of the system.
///
/// # Arguments
/// * `address` - Address to resolve, like `http://10.0.0.5:11420` or `http://sakura-1:11420`
///
/// # Returns
/// * `Ok(Ipv4Addr)` with the address of the host
/// * `Err(ErrorResponse)` if the host has no ipv4-address or can not be resolved
async fn resolve_address(address: &str) -> Result<Ipv4Addr, ErrorResponse> {
    let host = address_host(address).to_string();

    // an address, which is already an ip-address, doesn't have to be resolved
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return Ok(ip);
    }

    // the resolver of the standard-library blocks, so it is not called within the async-runtime
    let lookup_host = host.clone();
    let resolved = actix_web::rt::task::spawn_blocking(move || {
        (lookup_host.as_str(), 0u16)
            .to_socket_addrs()
            .map(|mut addrs| {
                addrs.find_map(|addr| match addr {
                    SocketAddr::V4(addr) => Some(*addr.ip()),
                    SocketAddr::V6(_) => None,
                })
            })
    })
    .await;

    match resolved {
        Ok(Ok(Some(ip))) => Ok(ip),
        Ok(Ok(None)) => {
            log::error!("Host '{host}' of address '{address}' has no ipv4-address.");
            Err(ErrorResponse::InternalError("Internal Error".to_string()))
        }
        Ok(Err(e)) => {
            log::error!("Failed to resolve host '{host}' of address '{address}': {e}");
            Err(ErrorResponse::InternalError("Internal Error".to_string()))
        }
        Err(e) => {
            log::error!("Failed to run the resolver for host '{host}': {e}");
            Err(ErrorResponse::InternalError("Internal Error".to_string()))
        }
    }
}

/// Splits the scheme from an address
///
/// # Arguments
/// * `address` - Address to split, like `http://127.0.0.1:10419`
///
/// # Returns
/// The scheme including the separator, like `http://`, and the rest of the address. The scheme is
/// empty, if the address has none.
fn split_scheme(address: &str) -> (&str, &str) {
    match address.find("://") {
        Some(position) => address.split_at(position + 3),
        None => ("", address),
    }
}

/// Reads the host of an address without its scheme, port and path
///
/// # Arguments
/// * `address` - Address to read the host from, like `http://127.0.0.1:10419`
///
/// # Returns
/// The host of the address, like `127.0.0.1`
fn address_host(address: &str) -> &str {
    let (_, rest) = split_scheme(address);
    let host = rest.split('/').next().unwrap_or(rest);

    match host.rsplit_once(':') {
        Some((host, _)) => host,
        None => host,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn torii_endpoint(internal_address: &str) -> Endpoint {
        Endpoint {
            public_address: "http://10.0.0.254:11419".to_string(),
            internal_address: internal_address.to_string(),
        }
    }

    #[test]
    fn test_address_host() {
        assert_eq!(address_host("http://10.0.0.5:10419"), "10.0.0.5");
        assert_eq!(address_host("https://vmm-gateway:10419"), "vmm-gateway");
        assert_eq!(address_host("10.0.0.5:10419"), "10.0.0.5");
        assert_eq!(address_host("http://10.0.0.5"), "10.0.0.5");
    }

    #[test]
    fn test_torii_of_host() {
        // the torii of the host keeps the scheme and the port of the torii of the endpoints
        let torii = torii_endpoint("http://10.0.0.254:10419");
        let host_torii = torii_of_host(&torii, "http://10.0.0.5:11420").unwrap();
        assert_eq!(host_torii.internal_address, "http://10.0.0.5:10419");

        // in a single-torii setup both addresses point to the same torii
        let host_torii = torii_of_host(&torii, "http://10.0.0.254:11420").unwrap();
        assert_eq!(host_torii.internal_address, "http://10.0.0.254:10419");
    }

    #[test]
    fn test_torii_of_host_without_port() {
        let torii = torii_endpoint("http://10.0.0.254");
        assert!(torii_of_host(&torii, "http://10.0.0.5:11420").is_err());
    }

    #[actix_rt::test]
    async fn test_resolve_address() {
        // an address, which already contains an ip-address, is taken as it is
        assert_eq!(
            resolve_address("http://10.0.0.5:11420").await.unwrap(),
            Ipv4Addr::new(10, 0, 0, 5)
        );

        // a host, which is addressed by a dns-name, is resolved
        assert_eq!(
            resolve_address("http://localhost:11420").await.unwrap(),
            Ipv4Addr::LOCALHOST
        );

        // an address without a host can not be resolved
        assert!(resolve_address("http://:11420").await.is_err());
    }
}
