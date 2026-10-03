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

//! Parsing and translation of the packet filter include-lists.
//!
//! A filter is kept twice: as the textual rules the control plane hands out and
//! takes back (`RouteFilterRules`), and as the flat `RouteFilter` struct the
//! eBPF datapath reads. This module owns the conversion between the two. IP and
//! port ranges arrive already parsed and normalised, see `IpRangeRule` and
//! `PortRangeRule`.
//!
//! Every filter belongs to one direction of the address of a VM within a tenant
//! (`FilterKey`). The ingress filter is stored under the route key of the route
//! towards the VM, the egress filter under the ifindex of the TAP device the VM
//! sends on. Both are only accepted for an address this gateway has a route
//! for, and both die with that route.

use torii_common::{
    FILTER_MAX_IP_RANGES, FILTER_MAX_PORT_RANGES, IpRange, PortRange, RouteFilter, RouteKey,
};

use ainari_api_structs::network_filter_structs::*;

use crate::core::models::{FilterKey, Route, RouteFilterPod, RouteKeyPod};
use crate::core::state::GatewayState;
use crate::core::utils::get_ifindex;

use ainari_api::errors::ErrorResponse;

/// Where the datapath keeps the filter of one `FilterKey`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterSlot {
    /// Entry of `FILTER_MAP`, keyed by the `(vni, destination)` key of the route towards the VM
    Ingress(RouteKey),
    /// Entry of `EGRESS_FILTER_MAP`, keyed by the ifindex of the TAP device of the VM
    Egress(u32),
}

/// Translates the textual rules of a route into the struct the datapath reads.
///
/// # Arguments
/// * `rules` - The include-lists currently attached to the route
///
/// # Returns
/// A `Result` holding the populated `RouteFilter`, or an error when one of the
/// lists exceeds the capacity of the eBPF map value
pub fn build_route_filter(rules: &RouteFilterRules) -> Result<RouteFilter, String> {
    if rules.ip_ranges.len() > FILTER_MAX_IP_RANGES {
        return Err(format!(
            "A route filter holds at most {} IP ranges",
            FILTER_MAX_IP_RANGES
        ));
    }
    if rules.ports.len() > FILTER_MAX_PORT_RANGES {
        return Err(format!(
            "A route filter holds at most {} port ranges",
            FILTER_MAX_PORT_RANGES
        ));
    }

    let mut filter = RouteFilter::empty();
    filter.ip_range_count = rules.ip_ranges.len() as u32;
    filter.port_range_count = rules.ports.len() as u32;

    for (slot, rule) in filter.ip_ranges.iter_mut().zip(rules.ip_ranges.iter()) {
        *slot = IpRange {
            start: u32::from(rule.first),
            end: u32::from(rule.last),
        };
    }
    for (slot, rule) in filter.port_ranges.iter_mut().zip(rules.ports.iter()) {
        *slot = PortRange {
            start: rule.first,
            end: rule.last,
        };
    }

    Ok(filter)
}

/// Resolves the place in the datapath, where one direction of the filter of a route is stored.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `route` - The route towards the address the filter belongs to
/// * `direction` - The direction of the filter
///
/// # Returns
/// The `FilterSlot` of the filter, or `None` for an egress filter of a route, which doesn't lead
/// to a TAP device of this gateway: there is no VM, whose traffic could be filtered.
pub fn slot_of_route(
    st: &GatewayState,
    route: &Route,
    direction: FilterDirection,
) -> Option<FilterSlot> {
    match direction {
        FilterDirection::Ingress => Some(FilterSlot::Ingress(RouteKey::new(
            route.vni,
            u32::from(route.dest_ip),
        ))),
        FilterDirection::Egress => {
            if !st.taps.contains_key(route.target_iface.as_str()) {
                return None;
            }
            match get_ifindex(&route.target_iface) {
                0 => None,
                ifindex => Some(FilterSlot::Egress(ifindex)),
            }
        }
    }
}

/// Resolves a filter to the place in the datapath, where it is stored.
///
/// The filter API addresses a filter by the tenant and the address of a VM, while the eBPF maps
/// are keyed by the route key of the route towards the VM and by the ifindex of its TAP device -
/// this is the one place that translation happens.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `key` - The tenant, the address and the direction of the filter
///
/// # Returns
/// The `FilterSlot` of the filter, or an error message, if this gateway has no route to the
/// address or, for an egress filter, the route doesn't lead to a TAP device
pub fn filter_slot(st: &GatewayState, key: &FilterKey) -> Result<FilterSlot, String> {
    // more than one route may lead to the same address, but only one of them has to lead to the
    // TAP device of the VM
    let mut routes = st
        .routes
        .values()
        .filter(|route| route.vni == key.vni && route.dest_ip == key.ip)
        .peekable();
    if routes.peek().is_none() {
        return Err(format!(
            "No route to {} in tenant {} on this gateway",
            key.ip, key.vni
        ));
    }

    routes
        .find_map(|route| slot_of_route(st, route, key.direction))
        .ok_or_else(|| {
            format!(
                "{} in tenant {} is not behind a TAP device of this gateway",
                key.ip, key.vni
            )
        })
}

/// Commits a new set of include-lists for one filter.
///
/// The translation into the eBPF representation happens first, so a filter that
/// does not fit into the map value is rejected before anything is changed.
/// A filter whose lists are both empty is removed from its map entirely:
/// no entry means no restriction, which is exactly what an empty include-list
/// is supposed to express - and it saves the datapath a lookup per packet.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `key` - The tenant, the address and the direction of the filter
/// * `slot` - The place in the datapath, where the filter is stored
/// * `rules` - The include-lists the filter should have from now on
///
/// # Returns
/// A `Result` that is `Ok(())` once both the eBPF map and the bookkeeping have
/// been updated, or an error message with nothing changed
pub fn apply_filter(
    st: &mut GatewayState,
    key: FilterKey,
    slot: FilterSlot,
    rules: RouteFilterRules,
) -> Result<(), String> {
    if rules.is_empty() {
        remove_from_datapath(st, slot);
        st.filters.remove(&key);
        return Ok(());
    }

    let filter = RouteFilterPod(build_route_filter(&rules)?);
    let result = match slot {
        FilterSlot::Ingress(route_key) => st.filter_map.insert(RouteKeyPod(route_key), filter, 0),
        FilterSlot::Egress(ifindex) => st.egress_filter_map.insert(ifindex, filter, 0),
    };
    result.map_err(|_| "eBPF Map error (filter)".to_string())?;
    st.filters.insert(key, rules);
    Ok(())
}

/// Removes a filter from its eBPF map, which makes the traffic unfiltered again.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `slot` - The place in the datapath, where the filter is stored
fn remove_from_datapath(st: &mut GatewayState, slot: FilterSlot) {
    match slot {
        FilterSlot::Ingress(route_key) => {
            let _ = st.filter_map.remove(&RouteKeyPod(route_key));
        }
        FilterSlot::Egress(ifindex) => {
            let _ = st.egress_filter_map.remove(&ifindex);
        }
    }
}

/// Persists the packet filter, which was just applied, or rolls it back.
///
/// If `persist` fails, the include-lists the filter had before are applied again, so the
/// datapath never holds a filter the database doesn't know about.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `key` - The tenant, the address and the direction of the filter
/// * `slot` - The place in the datapath, where the filter is stored
/// * `previous` - The include-lists the filter had before
/// * `persist` - Writes the current include-lists of the filter to the database
///
/// # Returns
/// `Ok(())` once the filter is persisted, otherwise the error of `persist`
pub fn persist_filter(
    st: &mut GatewayState,
    key: FilterKey,
    slot: FilterSlot,
    previous: RouteFilterRules,
    persist: impl FnOnce(&RouteFilterRules) -> Result<(), ErrorResponse>,
) -> Result<(), ErrorResponse> {
    let rules = st.filters.get(&key).cloned().unwrap_or_default();
    if let Err(e) = persist(&rules) {
        if let Err(rollback_err) = apply_filter(st, key, slot, previous) {
            log::error!(
                "Failed to roll back the {} packet-filter of {} in tenant {}: {rollback_err}",
                key.direction,
                key.ip,
                key.vni
            );
        }
        return Err(e);
    }
    Ok(())
}

/// Builds the response for one filter out of the bookkeeping.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `key` - The tenant, the address and the direction of the filter
///
/// # Returns
/// The `FilterResp` with the current include-lists, which are empty for an unfiltered address
pub fn filter_resp(st: &GatewayState, key: FilterKey) -> FilterResp {
    FilterResp {
        vni: key.vni,
        ip: key.ip,
        direction: key.direction,
        filter: st.filters.get(&key).cloned().unwrap_or_default(),
    }
}

/// Moves the filters of a route along with an update of the route.
///
/// A route that changed its destination *or its tenant* has to take its
/// filters with it, otherwise the new key would be reachable unfiltered while
/// the old one keeps an orphaned entry behind. A route that changed its target
/// interface has to move its egress filter to the new TAP device. An egress
/// filter, whose route doesn't lead to a TAP device anymore, has no VM to guard
/// and is dropped.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `from` - The version of the route, which was programmed before
/// * `to` - The version of the route, which is programmed from now on
///
/// # Returns
/// `Ok(())` once the filters are moved, or an error message if one of them could not be
/// programmed again
pub fn move_filters(st: &mut GatewayState, from: &Route, to: &Route) -> Result<(), String> {
    for direction in [FilterDirection::Ingress, FilterDirection::Egress] {
        let from_key = FilterKey {
            vni: from.vni,
            ip: from.dest_ip,
            direction,
        };
        let to_key = FilterKey {
            vni: to.vni,
            ip: to.dest_ip,
            direction,
        };
        let from_slot = slot_of_route(st, from, direction);
        let to_slot = slot_of_route(st, to, direction);
        if from_key == to_key && from_slot == to_slot {
            continue;
        }

        let rules = st.filters.remove(&from_key);
        if let Some(from_slot) = from_slot {
            remove_from_datapath(st, from_slot);
        }

        let Some(rules) = rules else {
            continue;
        };
        match to_slot {
            Some(to_slot) => apply_filter(st, to_key, to_slot, rules)?,
            None => log::warn!(
                "Dropped the {direction} packet-filter of {} in tenant {}, because its route \
                 doesn't lead to a TAP device anymore",
                to.dest_ip,
                to.vni
            ),
        }
    }

    Ok(())
}

/// Removes both filters of a route from the bookkeeping and from the datapath.
///
/// The filters guard the route and the VM behind it, so they die with the route.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `route` - The route, which is removed
pub fn remove_filters_of_route(st: &mut GatewayState, route: &Route) {
    for direction in [FilterDirection::Ingress, FilterDirection::Egress] {
        if let Some(slot) = slot_of_route(st, route, direction) {
            remove_from_datapath(st, slot);
        }
        st.filters.remove(&FilterKey {
            vni: route.vni,
            ip: route.dest_ip,
            direction,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rules_translate_to_a_filter_that_allows_everything() {
        let filter = build_route_filter(&RouteFilterRules::default()).unwrap();
        assert_eq!(filter.ip_range_count, 0);
        assert_eq!(filter.port_range_count, 0);
    }

    #[test]
    fn rules_are_copied_into_the_ebpf_representation() {
        let rules = RouteFilterRules {
            ip_ranges: vec!["10.0.0.0/24".parse().unwrap()],
            ports: vec!["5000-5100".parse().unwrap()],
        };
        let filter = build_route_filter(&rules).unwrap();

        assert_eq!(filter.ip_range_count, 1);
        assert_eq!(filter.ip_ranges[0].start, 0x0a000000);
        assert_eq!(filter.ip_ranges[0].end, 0x0a0000ff);
        assert_eq!(filter.port_range_count, 1);
        assert_eq!(filter.port_ranges[0].start, 5000);
        assert_eq!(filter.port_ranges[0].end, 5100);
    }

    #[test]
    fn a_list_longer_than_the_map_value_is_rejected() {
        let rules = RouteFilterRules {
            ip_ranges: (0..=FILTER_MAX_IP_RANGES)
                .map(|i| format!("10.0.0.{}", i).parse().unwrap())
                .collect(),
            ports: Vec::new(),
        };
        assert!(build_route_filter(&rules).is_err());
    }
}
