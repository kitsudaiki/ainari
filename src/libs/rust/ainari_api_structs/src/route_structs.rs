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

use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::common_structs::default_vni;

/// Request to create or update one route.
///
/// `vni` names the tenant `dest_ip` is valid in. Two routes may carry the same
/// destination as long as their tenants differ - that is the entire point of the
/// field, and what allows two VMs with the same address to be hosted side by
/// side. Leaving it out puts the route into the shared tenant 0.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RouteReq {
    pub dest_ip: Ipv4Addr,
    pub target_iface: String,
    #[serde(default = "default_vni")]
    pub vni: u32,
    #[serde(default)]
    pub gateway_ip: Option<Ipv4Addr>,
    #[serde(default)]
    pub next_hop_ip: Option<Ipv4Addr>,
    #[serde(default)]
    pub next_hop_mac: Option<String>,
    #[serde(default)]
    pub encrypted: bool,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RouteListResp {
    pub routes: Vec<RouteResp>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RouteResp {
    pub uuid: Uuid,
    pub vni: u32,
    pub dest_ip: Ipv4Addr,
    pub target_iface: String,
    pub gateway_ip: Option<Ipv4Addr>,
    pub next_hop_ip: Option<Ipv4Addr>,
    pub next_hop_mac: Option<String>,
    pub encrypted: bool,
}
