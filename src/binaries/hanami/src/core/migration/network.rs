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

//! Moves the network of a virtual_machine between the gateways of two sakura-hosts.
//!
//! The virtual_machine keeps its address, its MAC-address and its TAP-device, only the torii,
//! which owns the TAP-device, changes. A move consists of three steps, which are the same for a
//! migration and for its rollback, only with source and target swapped:
//!
//! 1. `attach_to_host`: the torii of the new host gets the TAP-device, the route onto it, the
//!    packet-filters of the virtual_machine, the membership in the MLS-group of the network and
//!    the routes towards the other virtual_machines of the network.
//! 2. `point_to_host`: the torii at the edge and the torii of the other hosts of the network
//!    route the address of the virtual_machine to the new host.
//! 3. `release_host`: the torii of the old host forgets the virtual_machine and, if it was the
//!    last one of the network on that host, the network as well.
//!
//! Every step only changes, what differs, so it can be repeated.

use std::collections::HashSet;
use std::net::Ipv4Addr;

use uuid::Uuid;

use crate::config;
use crate::core::mls::{grant_membership, is_encrypted, network_encrypted, revoke_membership};
use crate::core::routing::{
    delete_routes_to, ensure_route, overlay_route, resolve_address, torii_of_host,
};
use crate::database::address_table::{self, AddressEntry};
use crate::database::host_table::HostEntry;
use crate::database::network_filter_table;
use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::route_structs::RouteReq;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::network_filter as network_filter_clients;
use ainari_clients::network_interface as network_interface_clients;
use ainari_common::config::{Endpoint, Endpoints};
use ainari_common::enums::DbError;
use ainari_common::error::AinariError;

/// The network of a virtual_machine, which is moved to another host
pub struct VmNetwork {
    /// The virtual_machine
    virtual_machine_uuid: Uuid,
    /// Address of the virtual_machine with its TAP-device, MAC-address and tenant
    address: AddressEntry,
    /// Whether the network is encrypted (see `network_encrypted`)
    encrypted: bool,
    /// Endpoints of the components, which contains the torii at the edge of the network
    endpoints: Endpoints,
    /// Address of the torii at the edge of the network
    external_torii_ip: Ipv4Addr,
}

impl VmNetwork {
    /// Collects everything, which is required to move the network of a virtual_machine.
    ///
    /// # Arguments
    /// * `virtual_machine_uuid` - The virtual_machine
    /// * `address` - Address of the virtual_machine
    /// * `endpoints` - Endpoints of the components
    ///
    /// # Returns
    /// * `Ok(VmNetwork)` on success
    /// * `Err(ErrorResponse)` if the network or the edge of the network can not be read
    pub async fn new(
        virtual_machine_uuid: Uuid,
        address: AddressEntry,
        endpoints: Endpoints,
    ) -> Result<Self, ErrorResponse> {
        let disabled = network_table::is_encryption_disabled(&address.network_uuid)
            .map_err(|e| map_db_uuid_get_delete_error("network", &address.network_uuid, e))?;
        let external_torii_ip = resolve_address(&endpoints.torii.internal_address).await?;

        Ok(Self {
            virtual_machine_uuid,
            address,
            encrypted: network_encrypted(disabled),
            endpoints,
            external_torii_ip,
        })
    }

    /// Prepares the torii of a host for the virtual_machine, before it is started there.
    ///
    /// The packet-filters are applied, before the virtual_machine can send anything, so it is
    /// never reachable without them.
    ///
    /// # Arguments
    /// * `host` - The host, which runs the virtual_machine afterwards
    /// * `context` - User context containing authentication information
    ///
    /// # Returns
    /// * `Ok(())` if the torii of the host is ready for the virtual_machine
    /// * `Err(ErrorResponse)` with an appropriate error on failure
    pub async fn attach_to_host(
        &self,
        host: &HostEntry,
        context: &UserContext,
    ) -> Result<(), ErrorResponse> {
        let torii = torii_of_host(&self.endpoints.torii, &host.address)?;
        let host_ip = resolve_address(&host.address).await?;

        network_interface_clients::register_tap(
            &torii,
            &context.token,
            &config::INTERNAL_API_KEY,
            &self.address.tap_name,
            self.address.vni,
            Some(self.address.mac_address.clone()),
            Some(self.address.internal_ip),
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response)?;

        // replaces a route towards the old host, which the torii has, if it runs other
        // virtual_machines of the network
        ensure_route(&torii, self.local_route(), context).await?;
        self.apply_network_filters(&torii, context).await?;

        if self.encrypted && host_ip != self.external_torii_ip {
            grant_membership(
                &self.endpoints,
                host,
                host_ip,
                &torii,
                self.address.vni,
                context,
            )
            .await?;
        }

        for (other, other_host_ip) in self.other_virtual_machines().await? {
            // the torii routes between its own TAP-devices
            if other_host_ip == host_ip {
                continue;
            }
            let encrypted = is_encrypted(
                self.encrypted,
                host_ip,
                other_host_ip,
                self.external_torii_ip,
            );
            ensure_route(
                &torii,
                overlay_route(other.internal_ip, other_host_ip, other.vni, encrypted),
                context,
            )
            .await?;
        }

        Ok(())
    }

    /// Routes the address of the virtual_machine on all other gateways to a host.
    ///
    /// # Arguments
    /// * `host` - The host, which runs the virtual_machine afterwards
    /// * `context` - User context containing authentication information
    ///
    /// # Returns
    /// * `Ok(())` if all gateways route the virtual_machine to the host
    /// * `Err(ErrorResponse)` with an appropriate error on failure
    pub async fn point_to_host(
        &self,
        host: &HostEntry,
        context: &UserContext,
    ) -> Result<(), ErrorResponse> {
        let host_ip = resolve_address(&host.address).await?;
        let ip = self.address.internal_ip;
        let vni = self.address.vni;

        // in a single-torii setup the edge of the network is the torii of the host, which got
        // the route onto the TAP-device already. The route from the edge is never encrypted.
        if host_ip != self.external_torii_ip {
            ensure_route(
                &self.endpoints.torii,
                overlay_route(ip, host_ip, vni, false),
                context,
            )
            .await?;
        }

        // the torii of every host, which runs another virtual_machine of the network, once. This
        // includes the old host, if it runs another virtual_machine of the network.
        let mut updated_hosts = HashSet::from([host_ip, self.external_torii_ip]);
        for (other, other_host_ip) in self.other_virtual_machines().await? {
            if !updated_hosts.insert(other_host_ip) {
                continue;
            }
            let torii = torii_of_host(&self.endpoints.torii, &other.host_address)?;
            let encrypted = is_encrypted(
                self.encrypted,
                host_ip,
                other_host_ip,
                self.external_torii_ip,
            );
            ensure_route(&torii, overlay_route(ip, host_ip, vni, encrypted), context).await?;
        }

        Ok(())
    }

    /// Removes the virtual_machine from the torii of a host, which doesn't run it anymore.
    ///
    /// The routes towards the other virtual_machines of the network and the membership in the
    /// MLS-group are only removed, if the host runs no other virtual_machine of the network.
    /// Otherwise `point_to_host` already replaced the route onto the TAP-device by a route to the
    /// new host. The TAP-device itself stays, like with the deletion of a virtual_machine.
    ///
    /// # Arguments
    /// * `host_address` - Address of the host, which doesn't run the virtual_machine anymore
    /// * `context` - User context containing authentication information
    ///
    /// # Returns
    /// * `Ok(())` if the torii of the host forgot the virtual_machine
    /// * `Err(ErrorResponse)` with an appropriate error on failure
    pub async fn release_host(
        &self,
        host_address: &str,
        context: &UserContext,
    ) -> Result<(), ErrorResponse> {
        let host_ip = resolve_address(host_address).await?;
        let torii = torii_of_host(&self.endpoints.torii, host_address)?;

        // The edge of the network keeps its routes towards the virtual_machines, which are the
        // way to them from outside, and a host keeps them for its other virtual_machines of the
        // network. Their route towards this one was changed by `point_to_host`, but the torii
        // keeps the ingress-filter of the address with the changed route. It would still filter
        // the traffic towards the virtual_machine, also after its filter was changed on the new
        // host, so it is removed here.
        let others = self.other_virtual_machines().await?;
        if host_ip == self.external_torii_ip
            || others
                .iter()
                .any(|(_, other_host_ip)| *other_host_ip == host_ip)
        {
            return self.clear_network_filters(&torii, context).await;
        }

        // the packet-filters of the virtual_machine are dropped together with the route onto
        // its TAP-device
        delete_routes_to(&torii, self.address.internal_ip, self.address.vni, context).await?;
        for (other, _) in &others {
            delete_routes_to(&torii, other.internal_ip, other.vni, context).await?;
        }

        if self.encrypted {
            revoke_membership(&self.endpoints, host_ip, self.address.vni).await?;
        }

        Ok(())
    }

    /// The route onto the TAP-device of the virtual_machine on the torii of its host
    fn local_route(&self) -> RouteReq {
        RouteReq {
            dest_ip: self.address.internal_ip,
            target_iface: self.address.tap_name.clone(),
            vni: self.address.vni,
            gateway_ip: None,
            next_hop_ip: None,
            next_hop_mac: None,
            encrypted: false,
        }
    }

    /// The other virtual_machines of the network together with the underlay-address of their
    /// host.
    ///
    /// Addresses without a virtual_machine, like a failed reservation, and entries of an old
    /// database without the address of their host are skipped, because they have no torii.
    async fn other_virtual_machines(&self) -> Result<Vec<(AddressEntry, Ipv4Addr)>, ErrorResponse> {
        let addresses = address_table::list_addresses_of_network(&self.address.network_uuid)
            .map_err(|e| {
                log::error!(
                    "Failed to get the addresses of network '{}': {e}",
                    self.address.network_uuid
                );
                ErrorResponse::InternalError("Internal Error".to_string())
            })?;

        let mut others = Vec::new();
        for other in addresses {
            if other.uuid == self.address.uuid
                || other.host_address.is_empty()
                || other.virtual_machine_uuid.is_none()
            {
                continue;
            }
            let other_host_ip = resolve_address(&other.host_address).await?;
            others.push((other, other_host_ip));
        }
        Ok(others)
    }

    /// Removes the packet-filters of the virtual_machine from a torii.
    ///
    /// A filter, which the torii can't find anymore, is gone already. This is always the case for
    /// the egress-filter on a torii, which doesn't route onto the TAP-device of the
    /// virtual_machine anymore, because it is bound to the TAP-device. It only filters the
    /// traffic of this TAP-device, which is not used anymore.
    async fn clear_network_filters(
        &self,
        torii: &Endpoint,
        context: &UserContext,
    ) -> Result<(), ErrorResponse> {
        for direction in [FilterDirection::Ingress, FilterDirection::Egress] {
            match network_filter_clients::clear_filter(
                torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                self.address.vni,
                &self.address.internal_ip,
                direction,
                config::CONFIG.skip_tls_verification,
            )
            .await
            {
                Ok(_) | Err(AinariError::NotFound(_)) => {}
                Err(e) => return Err(map_ainari_error_to_api_response(e)),
            }
        }
        Ok(())
    }

    /// Applies the packet-filters of the virtual_machine, which are stored in hanami, on a
    /// torii. Existing rules on the torii are replaced, so the call can be repeated.
    async fn apply_network_filters(
        &self,
        torii: &Endpoint,
        context: &UserContext,
    ) -> Result<(), ErrorResponse> {
        let ip = &self.address.internal_ip;
        let vni = self.address.vni;
        let skip_tls = config::CONFIG.skip_tls_verification;

        for direction in [FilterDirection::Ingress, FilterDirection::Egress] {
            network_filter_clients::clear_filter(
                torii,
                &context.token,
                &config::INTERNAL_API_KEY,
                vni,
                ip,
                direction,
                skip_tls,
            )
            .await
            .map_err(map_ainari_error_to_api_response)?;

            let entry = match network_filter_table::get_network_filter(
                &self.virtual_machine_uuid,
                direction,
                context,
            ) {
                Ok(entry) => entry,
                Err(DbError::NotFound) => continue,
                Err(e) => {
                    return Err(map_db_uuid_get_delete_error(
                        "network-filter",
                        &self.virtual_machine_uuid,
                        e,
                    ));
                }
            };

            let ranges = parse_rules::<IpRangeRule>(&entry.ip_range_list())?;
            if !ranges.is_empty() {
                network_filter_clients::add_filter_ip_range(
                    torii,
                    &context.token,
                    &config::INTERNAL_API_KEY,
                    vni,
                    ip,
                    direction,
                    ranges,
                    skip_tls,
                )
                .await
                .map_err(map_ainari_error_to_api_response)?;
            }

            let ports = parse_rules::<PortRangeRule>(&entry.port_list())?;
            if !ports.is_empty() {
                network_filter_clients::add_filter_port(
                    torii,
                    &context.token,
                    &config::INTERNAL_API_KEY,
                    vni,
                    ip,
                    direction,
                    ports,
                    skip_tls,
                )
                .await
                .map_err(map_ainari_error_to_api_response)?;
            }
        }

        Ok(())
    }
}

/// Parses the rules of a packet-filter, like they are stored in the database.
///
/// # Arguments
/// * `rules` - The rules as strings
///
/// # Returns
/// * `Ok(Vec<T>)` with the parsed rules
/// * `Err(ErrorResponse)` if a stored rule is invalid
fn parse_rules<T: std::str::FromStr<Err = String>>(
    rules: &[String],
) -> Result<Vec<T>, ErrorResponse> {
    rules
        .iter()
        .map(|rule| {
            rule.parse().map_err(|e| {
                log::error!("Invalid rule '{rule}' of a packet-filter in database: {e}");
                ErrorResponse::InternalError("Internal Error".to_string())
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rules() {
        let ranges = parse_rules::<IpRangeRule>(&["10.0.0.0/24".to_string()]).unwrap();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].first, Ipv4Addr::new(10, 0, 0, 0));
        assert_eq!(ranges[0].last, Ipv4Addr::new(10, 0, 0, 255));

        let ports =
            parse_rules::<PortRangeRule>(&["22".to_string(), "8000-8080".to_string()]).unwrap();
        assert_eq!((ports[1].first, ports[1].last), (8000, 8080));

        assert!(parse_rules::<PortRangeRule>(&[]).unwrap().is_empty());
        assert!(parse_rules::<IpRangeRule>(&["no-address".to_string()]).is_err());
    }
}
