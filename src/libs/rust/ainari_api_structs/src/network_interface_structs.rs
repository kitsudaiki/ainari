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

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TapRequest {
    pub tap_name: String,
    #[serde(default)]
    pub vm_mac: Option<String>,
    #[serde(default)]
    pub vm_ip: Option<Ipv4Addr>,
}

#[derive(Debug, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TapResponse {
    pub success: bool,
    pub message: String,
    pub tap_name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct IfaceConfigRequest {
    pub iface_name: String,
    pub ip_cidr: Option<String>,
    pub up: bool,
}
