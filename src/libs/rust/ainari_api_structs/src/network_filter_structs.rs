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

use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::net::Ipv4Addr;
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct FilterIpRangeReq {
    /// Single address, CIDR subnet or explicit `first-last` range
    #[schemars(with = "Vec<String>")]
    pub ranges: Vec<IpRangeRule>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct FilterPortReq {
    /// Single port or explicit `first-last` range
    #[schemars(with = "Vec<String>")]
    pub ports: Vec<PortRangeRule>,
}

#[derive(Debug, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct FilterResp {
    pub route_uuid: Uuid,
    pub dest_ip: Ipv4Addr,
    pub filter: RouteFilterRules,
}

/// Response payload for listing the packet filters of all routes.
#[derive(Debug, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct FilterListResponse {
    pub filters: Vec<FilterEntry>,
}

#[derive(Debug, Clone, Default, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct RouteFilterRules {
    pub ip_ranges: Vec<IpRangeRule>,
    pub ports: Vec<PortRangeRule>,
}

impl RouteFilterRules {
    /// Reports whether this route is unfiltered, i.e. both include-lists are empty.
    ///
    /// # Arguments
    /// None
    ///
    /// # Returns
    /// `true` when neither an IP range nor a port has been added yet
    pub fn is_empty(&self) -> bool {
        self.ip_ranges.is_empty() && self.ports.is_empty()
    }
}

/// One entry of the filter overview.
#[derive(Debug, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct FilterEntry {
    pub route_uuid: Uuid,
    pub dest_ip: Ipv4Addr,
    pub filter: RouteFilterRules,
}

#[derive(Debug, Clone, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct IpRangeRule {
    pub spec: String,
    pub first: Ipv4Addr,
    pub last: Ipv4Addr,
}

impl FromStr for IpRangeRule {
    type Err = String;

    fn from_str(spec: &str) -> Result<Self, Self::Err> {
        parse_ip_range(spec)
    }
}

/// Requests carry an IP range in its textual notation, so it is parsed right
/// while deserializing and a malformed entry rejects the whole request.
impl<'de> Deserialize<'de> for IpRangeRule {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let spec = String::deserialize(deserializer)?;
        spec.parse().map_err(serde::de::Error::custom)
    }
}

/// One entry of the port include-list of a route, as an inclusive range.
#[derive(Debug, Clone, Serialize, JsonSchema, ApiComponent, Validate)]
pub struct PortRangeRule {
    pub spec: String,
    pub first: u16,
    pub last: u16,
}

impl FromStr for PortRangeRule {
    type Err = String;

    fn from_str(spec: &str) -> Result<Self, Self::Err> {
        parse_port_range(spec)
    }
}

/// Requests carry a port range in its textual notation, so it is parsed right
/// while deserializing and a malformed entry rejects the whole request.
impl<'de> Deserialize<'de> for PortRangeRule {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let spec = String::deserialize(deserializer)?;
        spec.parse().map_err(serde::de::Error::custom)
    }
}

/// Renders the canonical text form of an address range.
///
/// The same range can be written in several ways, and the client is free to use
/// any of them. What comes back is always the shortest unambiguous form: a bare
/// address for a single host, CIDR notation whenever the range happens to be an
/// aligned block, and the explicit `first-last` form for everything else.
///
/// # Arguments
/// * `first` - First address of the range
/// * `last` - Last address of the range (inclusive)
///
/// # Returns
/// A `String` holding the canonical notation of the range
fn canonical_ip_spec(first: u32, last: u32) -> String {
    if first == last {
        return Ipv4Addr::from(first).to_string();
    }

    // A range is a subnet exactly when its size is a power of two and its first
    // address is aligned to that size.
    let size = u64::from(last) - u64::from(first) + 1;
    if size.is_power_of_two() && u64::from(first) % size == 0 {
        let prefix = 32 - size.trailing_zeros();
        return format!("{}/{}", Ipv4Addr::from(first), prefix);
    }

    format!("{}-{}", Ipv4Addr::from(first), Ipv4Addr::from(last))
}

/// Parses one entry of the IP include-list.
///
/// Three notations are accepted and all of them end up as an inclusive range:
///
/// * `10.0.0.7` - a single address
/// * `10.0.0.0/24` - a subnet, expanded to its first and last address
/// * `10.0.0.5-10.0.0.9` - an explicit range
///
/// # Arguments
/// * `spec` - The textual entry as it arrived from the client
///
/// # Returns
/// A `Result` with the parsed rule, or a message naming what was wrong with it
fn parse_ip_range(spec: &str) -> Result<IpRangeRule, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("Empty IP range".to_string());
    }

    let (first, last) = if let Some((addr, prefix)) = spec.split_once('/') {
        let addr: Ipv4Addr = addr
            .trim()
            .parse()
            .map_err(|_| format!("Invalid address in '{}'", spec))?;
        let prefix: u32 = prefix
            .trim()
            .parse()
            .map_err(|_| format!("Invalid prefix length in '{}'", spec))?;
        if prefix > 32 {
            return Err(format!("Prefix length out of range in '{}'", spec));
        }
        // A /0 mask cannot be produced by shifting a u32 by 32, so widen first.
        let mask = (!0u64 << (32 - prefix)) as u32;
        let network = u32::from(addr) & mask;
        (network, network | !mask)
    } else if let Some((from, to)) = spec.split_once('-') {
        let from: Ipv4Addr = from
            .trim()
            .parse()
            .map_err(|_| format!("Invalid start address in '{}'", spec))?;
        let to: Ipv4Addr = to
            .trim()
            .parse()
            .map_err(|_| format!("Invalid end address in '{}'", spec))?;
        if u32::from(from) > u32::from(to) {
            return Err(format!("Range '{}' ends before it starts", spec));
        }
        (u32::from(from), u32::from(to))
    } else {
        let addr: Ipv4Addr = spec
            .parse()
            .map_err(|_| format!("Invalid address '{}'", spec))?;
        (u32::from(addr), u32::from(addr))
    };

    Ok(IpRangeRule {
        spec: canonical_ip_spec(first, last),
        first: Ipv4Addr::from(first),
        last: Ipv4Addr::from(last),
    })
}

/// Parses one entry of the port include-list.
///
/// Accepts a single port (`22`) as well as a range (`8000-8100`).
///
/// # Arguments
/// * `spec` - The textual entry as it arrived from the client
///
/// # Returns
/// A `Result` with the parsed rule, or a message naming what was wrong with it
fn parse_port_range(spec: &str) -> Result<PortRangeRule, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("Empty port".to_string());
    }

    let (first, last) = if let Some((from, to)) = spec.split_once('-') {
        let from: u16 = from
            .trim()
            .parse()
            .map_err(|_| format!("Invalid start port in '{}'", spec))?;
        let to: u16 = to
            .trim()
            .parse()
            .map_err(|_| format!("Invalid end port in '{}'", spec))?;
        if from > to {
            return Err(format!("Port range '{}' ends before it starts", spec));
        }
        (from, to)
    } else {
        let port: u16 = spec
            .parse()
            .map_err(|_| format!("Invalid port '{}'", spec))?;
        (port, port)
    };

    if first == 0 {
        return Err(format!("Port 0 is not a usable port in '{}'", spec));
    }

    let canonical = if first == last {
        format!("{}", first)
    } else {
        format!("{}-{}", first, last)
    };

    Ok(PortRangeRule {
        spec: canonical,
        first,
        last,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses a spec and returns its canonical form together with its bounds.
    fn ip(spec: &str) -> (String, u32, u32) {
        let rule = parse_ip_range(spec).expect(spec);
        (rule.spec, u32::from(rule.first), u32::from(rule.last))
    }

    #[test]
    fn single_address_is_its_own_range() {
        assert_eq!(
            ip("10.0.0.7"),
            ("10.0.0.7".to_string(), 0x0a000007, 0x0a000007)
        );
    }

    #[test]
    fn subnet_expands_to_first_and_last_address() {
        assert_eq!(
            ip("10.0.0.0/24"),
            ("10.0.0.0/24".to_string(), 0x0a000000, 0x0a0000ff)
        );
        // Host bits are dropped, exactly like a router would.
        assert_eq!(ip("10.0.0.42/24").1, 0x0a000000);
        assert_eq!(ip("0.0.0.0/0"), ("0.0.0.0/0".to_string(), 0, u32::MAX));
        assert_eq!(
            ip("10.0.0.7/32"),
            ("10.0.0.7".to_string(), 0x0a000007, 0x0a000007)
        );
    }

    #[test]
    fn explicit_range_keeps_its_bounds() {
        assert_eq!(
            ip("10.0.0.5-10.0.0.9"),
            ("10.0.0.5-10.0.0.9".to_string(), 0x0a000005, 0x0a000009)
        );
    }

    #[test]
    fn a_range_that_is_a_subnet_is_reported_as_one() {
        // This is what makes removal independent of the notation used to add.
        assert_eq!(ip("10.0.0.0-10.0.0.255").0, "10.0.0.0/24");
    }

    #[test]
    fn broken_ip_specs_are_rejected() {
        assert!(parse_ip_range("").is_err());
        assert!(parse_ip_range("10.0.0.256").is_err());
        assert!(parse_ip_range("10.0.0.0/33").is_err());
        assert!(parse_ip_range("10.0.0.9-10.0.0.5").is_err());
    }

    #[test]
    fn requests_carry_ranges_in_textual_notation() {
        let req: FilterIpRangeReq =
            serde_json::from_str(r#"{"ranges": ["10.0.0.7", "10.0.0.0-10.0.0.255"]}"#).unwrap();
        assert_eq!(req.ranges[0].spec, "10.0.0.7");
        assert_eq!(req.ranges[1].spec, "10.0.0.0/24");

        assert!(serde_json::from_str::<FilterIpRangeReq>(r#"{"ranges": ["10.0.0.256"]}"#).is_err());
    }

    #[test]
    fn ports_accept_singles_and_ranges() {
        let single = parse_port_range("22").unwrap();
        assert_eq!(
            (single.spec.as_str(), single.first, single.last),
            ("22", 22, 22)
        );

        let range = parse_port_range("5000-5100").unwrap();
        assert_eq!(
            (range.spec.as_str(), range.first, range.last),
            ("5000-5100", 5000, 5100)
        );
    }

    #[test]
    fn broken_port_specs_are_rejected() {
        assert!(parse_port_range("").is_err());
        assert!(parse_port_range("0").is_err());
        assert!(parse_port_range("65536").is_err());
        assert!(parse_port_range("100-10").is_err());
    }

    #[test]
    fn requests_carry_ports_in_textual_notation() {
        let req: FilterPortReq = serde_json::from_str(r#"{"ports": ["22", "8000-8100"]}"#).unwrap();
        assert_eq!((req.ports[0].first, req.ports[0].last), (22, 22));
        assert_eq!((req.ports[1].first, req.ports[1].last), (8000, 8100));

        assert!(serde_json::from_str::<FilterPortReq>(r#"{"ports": ["0"]}"#).is_err());
    }
}
