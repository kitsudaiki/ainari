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
use validator::Validate;

use crate::common_structs::default_vni;

/// Request to create a TAP device and attach a VM to it.
///
/// `vni` places the port into a tenant. Everything the VM behind it sends takes
/// its tenant from this registration and from nowhere else, which is what lets
/// two VMs with the *same address* live on one host.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TapReq {
    pub tap_name: String,
    #[serde(default = "default_vni")]
    pub vni: u32,
    #[serde(default)]
    pub vm_mac: Option<String>,
    #[serde(default)]
    pub vm_ip: Option<Ipv4Addr>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TapResp {
    pub success: bool,
    pub message: String,
    pub tap_name: String,
    pub vni: u32,
}

/// Request to configure an already existing interface.
///
/// Besides the address and the link state this is also where a port is placed
/// into a tenant. `fip_port` marks the interface that faces the outside world:
/// only there does the datapath translate floating IPs, and only there may a
/// floating IP decide which tenant a packet belongs to. Marking a TAP as a
/// floating IP port would let the VM behind it reach every other tenant by
/// addressing a floating IP, so the two settings exclude each other in practice.
#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct IfaceConfigReq {
    pub iface_name: String,
    pub ip_cidr: Option<String>,
    pub up: bool,
    #[serde(default = "default_vni")]
    pub vni: u32,
    #[serde(default)]
    pub fip_port: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct IfaceConfigResp {
    pub iface_name: String,
    pub ip_cidr: Option<String>,
    pub up: bool,
    pub vni: u32,
    pub fip_port: bool,
}
