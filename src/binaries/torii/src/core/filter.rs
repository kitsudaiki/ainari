//! Parsing and translation of the packet filter include-lists.
//!
//! A route filter is kept twice: as the textual rules the control plane hands
//! out and takes back (`RouteFilterRules`), and as the flat `RouteFilter`
//! struct the eBPF datapath reads. This module owns the conversion between the
//! two. IP and port ranges arrive already parsed and normalised, see
//! `IpRangeRule` and `PortRangeRule`.

use std::net::Ipv4Addr;

use uuid::Uuid;

use torii_common::{FILTER_MAX_IP_RANGES, FILTER_MAX_PORT_RANGES, IpRange, PortRange, RouteFilter};

use ainari_api_structs::network_filter_structs::*;

use crate::core::models::RouteFilterPod;
use crate::core::state::GatewayState;

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

/// Resolves a route UUID to its destination address and its eBPF map key.
///
/// Every filter operation addresses a route by its UUID, while both eBPF maps
/// are keyed by the destination address of that route - this is the one place
/// that translation happens.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `route_uuid` - The UUID of the route, taken from the URL
///
/// # Returns
/// An `Option` with the destination address and the map key of the route, or
/// `None` when no such route exists
pub fn route_filter_key(st: &GatewayState, route_uuid: &Uuid) -> Option<(Ipv4Addr, u32)> {
    let route = st.routes.get(route_uuid)?;
    Some((route.dest_ip, u32::from(route.dest_ip)))
}

/// Commits a new set of include-lists for one route.
///
/// The translation into the eBPF representation happens first, so a filter that
/// does not fit into the map value is rejected before anything is changed.
/// A route whose lists are both empty is removed from the filter map entirely:
/// no entry means no restriction, which is exactly what an empty include-list
/// is supposed to express - and it saves the datapath a lookup per packet.
///
/// # Arguments
/// * `st` - The locked gateway state
/// * `route_uuid` - The UUID of the route the filter belongs to
/// * `dest_key` - The eBPF map key of that route
/// * `rules` - The include-lists the route should have from now on
///
/// # Returns
/// A `Result` that is `Ok(())` once both the eBPF map and the bookkeeping have
/// been updated, or an error message with nothing changed
pub fn apply_filter(
    st: &mut GatewayState,
    route_uuid: Uuid,
    dest_key: u32,
    rules: RouteFilterRules,
) -> Result<(), String> {
    if rules.is_empty() {
        let _ = st.filter_map.remove(&dest_key);
        st.filters.remove(&route_uuid);
        return Ok(());
    }

    let filter = build_route_filter(&rules)?;
    st.filter_map
        .insert(dest_key, RouteFilterPod(filter), 0)
        .map_err(|_| "eBPF Map error (filter)".to_string())?;
    st.filters.insert(route_uuid, rules);
    Ok(())
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
