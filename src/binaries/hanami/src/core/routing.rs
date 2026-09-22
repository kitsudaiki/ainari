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

use crate::config;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::RouteReq;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::route as route_clients;
use ainari_common::config::Endpoint;

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
pub fn torii_of_host(
    torii_endpoint: &Endpoint,
    host_address: &str,
) -> Result<Endpoint, ErrorResponse> {
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
/// * `address` - Address to resolve, like `http://10.0.0.5:11420` or `http://sakura:11420`
///
/// # Returns
/// * `Ok(Ipv4Addr)` with the address of the host
/// * `Err(ErrorResponse)` if the host has no ipv4-address or can not be resolved
pub async fn resolve_address(address: &str) -> Result<Ipv4Addr, ErrorResponse> {
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

/// Creates a route on a torii towards a virtual_machine, which runs on another host
///
/// The target-interface is left empty, so the torii sends the traffic over the interface of its
/// own underlay to the gateway of the host, which runs the virtual_machine.
///
/// # Arguments
/// * `torii` - Endpoint of the torii, which gets the route
/// * `dest_ip` - Internal address of the virtual_machine, which the route leads to
/// * `gateway_ip` - Underlay-address of the sakura-host, which runs that virtual_machine
/// * `vni` - Tenant of the network, which the destination belongs to
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the route is created
/// * `Err(ErrorResponse)` with an appropriate error on failure
pub async fn create_overlay_route(
    torii: &Endpoint,
    dest_ip: Ipv4Addr,
    gateway_ip: Ipv4Addr,
    vni: u32,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    route_clients::create_route(
        torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &RouteReq {
            dest_ip,
            target_iface: String::new(),
            vni,
            gateway_ip: Some(gateway_ip),
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

/// Deletes all routes of a torii, which lead to an address within one tenant
///
/// The routes are addressed by their uuid, which is not stored in hanami, so the routes of the
/// torii are listed and the ones towards the address are deleted. A torii without such a route
/// is left as it is. The tenant is part of the comparison, because the very same address may be
/// routed in another tenant as well, and that route belongs to a different network.
///
/// # Arguments
/// * `torii` - Endpoint of the torii, whose routes are deleted
/// * `dest_ip` - Internal address of the virtual_machine, whose routes are deleted
/// * `vni` - Tenant of the network, which the address belongs to
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if all routes towards the address are gone
/// * `Err(ErrorResponse)` with an appropriate error on failure
pub async fn delete_routes_to(
    torii: &Endpoint,
    dest_ip: Ipv4Addr,
    vni: u32,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    let routes = route_clients::list_route(
        torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    for route in routes.routes {
        if route.dest_ip != dest_ip || route.vni != vni {
            continue;
        }

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

    Ok(())
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
