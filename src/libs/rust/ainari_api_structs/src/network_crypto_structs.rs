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
use std::str::FromStr;

use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

/// Identity of a gateway within the MLS-groups of the networks
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent)]
pub struct MlsIdentityResp {
    /// Identity of the gateway, which is its underlay-address
    pub client_id: String,
    /// Base64-encoded public MLS signature-key of the gateway. hanami pins it and names it in
    /// every membership-grant of the gateway.
    pub signature_key: String,
}

/// Epoch of a network on a gateway: the generation of the group and the epoch within the group
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
pub struct MlsEpochIdResp {
    pub generation: u64,
    pub epoch: u64,
}

/// Step of the key-change of a pair, which the pair waits for
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "snake_case")]
pub enum MlsPairWaitingFor {
    /// The incoming keys of the target are installed
    Ingress,
    /// The peer tells, that it can receive with the target
    PeerReady,
    /// The outgoing traffic switches to the target
    Egress,
    /// The peer tells, that it sends with the target
    PeerSwitched,
    /// Packets still arrive with old incoming keys
    OldIngressInUse,
    /// The pair has arrived
    Nothing,
}

/// Keys between this gateway and one peer gateway in a network
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, JsonSchema, ApiComponent)]
pub struct MlsPairStatusResp {
    /// Identity (underlay-address) of the peer gateway
    pub peer: String,
    /// Epoch of the outgoing keys
    pub egress: Option<MlsEpochIdResp>,
    /// Epoch, which the pair is moving to
    pub target: Option<MlsEpochIdResp>,
    /// Epochs of the incoming keys
    pub ingress: Vec<MlsEpochIdResp>,
    /// Old epochs, whose incoming keys are removed, once no packet arrives with them anymore
    pub retiring: Vec<MlsEpochIdResp>,
    pub waiting_for: MlsPairWaitingFor,
    /// Unix-time in seconds, since which the peer didn't join the current group
    pub stale_since: Option<i64>,
    /// Why the last step of the pair failed, if it did
    pub last_error: Option<String>,
}

/// Keys of a network on this gateway
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, JsonSchema, ApiComponent)]
pub struct MlsNetworkStatusResp {
    /// Tenant of the network
    pub vni: u32,
    /// Generation of the current group
    pub generation: u64,
    /// Current epoch of the group, if the gateway is a member
    pub epoch: Option<u64>,
    pub pairs: Vec<MlsPairStatusResp>,
}

/// State of the key-exchange of a gateway
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, JsonSchema, ApiComponent)]
pub struct MlsStatusResp {
    /// Identity of the gateway, if it has one yet
    pub client_id: Option<String>,
    pub networks: Vec<MlsNetworkStatusResp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_waiting_step_travels_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&MlsPairWaitingFor::OldIngressInUse).unwrap(),
            "\"old_ingress_in_use\""
        );
    }

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
