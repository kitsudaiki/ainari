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

use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;

use ainari_common::secret::Secret;
use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// The direction a crypto-key protects, which is what separates an outbound
/// Security Association from an inbound one.
///
/// A key is only ever one of the two, so the direction is part of the type
/// instead of a string that every handler would have to re-validate.
#[derive(
    Debug,
    Deserialize,
    Serialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    JsonSchema,
    ApiComponent,
)]
#[serde(rename_all = "lowercase")]
pub enum CryptoDirection {
    /// Protects the traffic leaving this gateway
    Egress,
    /// Protects the traffic arriving at this gateway
    Ingress,
}

impl fmt::Display for CryptoDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CryptoDirection::Egress => "egress",
            CryptoDirection::Ingress => "ingress",
        };
        write!(f, "{s}")
    }
}

impl FromStr for CryptoDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "egress" => Ok(CryptoDirection::Egress),
            "ingress" => Ok(CryptoDirection::Ingress),
            other => Err(format!(
                "Unknown direction '{other}', expected egress or ingress"
            )),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoKeyReq {
    pub direction: CryptoDirection,
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub spi: u32,
    pub key: Secret,
}

/// Addresses one installed key, which is identified by its direction together
/// with its Security-Parameter-Index.
#[derive(Debug, Deserialize, JsonSchema, ApiComponent)]
pub struct CryptoKeyPath {
    pub direction: CryptoDirection,
    pub spi: u32,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoKeyListResp {
    pub keys: Vec<CryptoKeyResp>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoKeyResp {
    pub direction: CryptoDirection,
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub spi: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoToggleReq {
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    #[serde(default)]
    pub peer_gateway_ip: Option<Ipv4Addr>,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoToggleResp {
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    #[serde(default)]
    pub peer_gateway_ip: Option<Ipv4Addr>,
    pub enabled: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ConnectionListResp {
    pub connections: Vec<ConnectionResp>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ConnectionResp {
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub enabled: bool,
    pub active_egress_spi: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_direction_travels_as_its_lowercase_name() {
        // The wire-format is what a gateway matches on, in the body of a request
        // as well as in the path of a URL.
        assert_eq!(
            serde_json::to_string(&CryptoDirection::Egress).unwrap(),
            "\"egress\""
        );
        assert_eq!(CryptoDirection::Ingress.to_string(), "ingress");
        assert_eq!(
            "ingress".parse::<CryptoDirection>().unwrap(),
            CryptoDirection::Ingress
        );
    }

    #[test]
    fn anything_but_the_two_directions_is_rejected() {
        assert!("outbound".parse::<CryptoDirection>().is_err());
        assert!(serde_json::from_str::<CryptoDirection>("\"Egress\"").is_err());
    }
}
