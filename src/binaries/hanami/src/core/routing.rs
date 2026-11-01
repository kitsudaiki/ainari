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

use crate::config;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::route_structs::{RouteReq, RouteResp};
use ainari_api_structs::user_context::UserContext;
use ainari_clients::network_interface as network_interface_clients;
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
            ainari_api::report_internal_error!(
                None,
                "Torii-address '{torii_address}' has no port."
            );
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    Ok(Endpoint {
        internal_address: format!("{scheme}{}:{port}", address_host(host_address)),
        public_address: torii_endpoint.public_address.clone(),
    })
}

/// Asks a torii for the address of its underlay
///
/// The underlay is not derived from the address of the api of the torii, so it can be a network of
/// its own. The other gateways send the traffic of the virtual_machines to this address, and it is
/// the identity of the torii within the MLS-groups of the networks.
///
/// # Arguments
/// * `torii` - Endpoint of the torii, which is asked
/// * `edge` - Endpoint of the torii at the edge of the network
/// * `context` - User context containing authentication information, `None` for background-jobs
///
/// # Returns
/// * `Ok(Ipv4Addr)` with the address of the underlay
/// * `Err(ErrorResponse)` if the torii can not be asked or has no underlay, see
///   `underlay_or_edge`
pub async fn underlay_address(
    torii: &Endpoint,
    edge: &Endpoint,
    context: Option<&UserContext>,
) -> Result<Ipv4Addr, ErrorResponse> {
    let token = context
        .map(|context| context.token.clone())
        .unwrap_or_default();
    let underlay = network_interface_clients::get_underlay(
        torii,
        &token,
        &config::INTERNAL_API_KEY,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(|e| map_ainari_error_to_api_response(e, context))?;

    underlay_or_edge(underlay.address, torii, edge)
}

/// Asks the torii of a sakura-host for the address of its underlay
///
/// # Arguments
/// * `edge` - Endpoint of the torii at the edge of the network
/// * `host_address` - Address of the sakura-host
/// * `context` - User context containing authentication information, `None` for background-jobs
///
/// # Returns
/// * `Ok(Ipv4Addr)` with the address of the underlay, see `underlay_address`
/// * `Err(ErrorResponse)` if the torii can not be asked or has no underlay
pub async fn host_underlay_address(
    edge: &Endpoint,
    host_address: &str,
    context: Option<&UserContext>,
) -> Result<Ipv4Addr, ErrorResponse> {
    underlay_address(&torii_of_host(edge, host_address)?, edge, context).await
}

/// Asks the torii at the edge of the network for the address of its underlay
///
/// # Arguments
/// * `edge` - Endpoint of the torii at the edge of the network
/// * `context` - User context containing authentication information, `None` for background-jobs
///
/// # Returns
/// * `Ok(Ipv4Addr)` with the address of the underlay, see `underlay_address`
/// * `Err(ErrorResponse)` if the torii can not be asked
pub async fn edge_underlay_address(
    edge: &Endpoint,
    context: Option<&UserContext>,
) -> Result<Ipv4Addr, ErrorResponse> {
    underlay_address(edge, edge, context).await
}

/// Checks the underlay-address, which a torii reported.
///
/// A torii without underlay is only valid as the single torii of a single-node setup, which is
/// the torii of the host and the edge of the network at the same time. It gets the unspecified
/// address, so it is the same as the edge for all comparisons, and it is never the target of a
/// route. Every other torii without underlay can't be reached by the other gateways, for example
/// because the interface of its underlay has no address yet.
///
/// # Arguments
/// * `address` - Underlay-address, which the torii reported
/// * `torii` - Endpoint of the torii
/// * `edge` - Endpoint of the torii at the edge of the network
///
/// # Returns
/// * `Ok(Ipv4Addr)` with the address of the underlay
/// * `Err(ErrorResponse)` if the torii has no underlay
fn underlay_or_edge(
    address: Option<Ipv4Addr>,
    torii: &Endpoint,
    edge: &Endpoint,
) -> Result<Ipv4Addr, ErrorResponse> {
    match address {
        Some(address) => Ok(address),
        None if torii.internal_address == edge.internal_address => Ok(Ipv4Addr::UNSPECIFIED),
        None => {
            ainari_api::report_internal_error!(
                None,
                "The torii '{}' has no address on its underlay",
                torii.internal_address
            );
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
/// * `encrypted` - Protect the route with IPsec, whose keys the torii derives from the MLS-group
///   of the network
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
    encrypted: bool,
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
            encrypted,
        },
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;

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
    .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;

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
        .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;
    }

    Ok(())
}

/// Builds the request of a route towards a virtual_machine on another host. The target-interface
/// is left empty, so the torii uses the interface of its own underlay.
pub fn overlay_route(
    dest_ip: Ipv4Addr,
    gateway_ip: Ipv4Addr,
    vni: u32,
    encrypted: bool,
) -> RouteReq {
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
pub async fn list_routes(
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
    .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?
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
/// * `context` - User context containing authentication information
///
/// # Returns
/// `Ok(())` once the torii has the route
pub async fn ensure_route(
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
            .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;
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
            .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;
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
            .map_err(|e| map_ainari_error_to_api_response(e, Some(context)))?;
        }
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
    use uuid::Uuid;

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

    #[test]
    fn test_underlay_or_edge() {
        let edge = torii_endpoint("http://10.0.0.254:10419");
        let host_torii = torii_endpoint("http://10.0.0.5:10419");
        let address = Ipv4Addr::new(192, 168, 57, 12);

        // a reported address is taken as it is
        assert_eq!(
            underlay_or_edge(Some(address), &host_torii, &edge).unwrap(),
            address
        );
        assert_eq!(
            underlay_or_edge(Some(address), &edge, &edge).unwrap(),
            address
        );

        // the single torii of a single-node setup is the edge and has no underlay
        assert_eq!(
            underlay_or_edge(None, &edge, &edge).unwrap(),
            Ipv4Addr::UNSPECIFIED
        );

        // the torii of another host without underlay can't be reached
        assert!(underlay_or_edge(None, &host_torii, &edge).is_err());
    }

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
