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
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct FloatingIpCreateReq {
    pub name: String,
    pub network_uuid: Uuid,
    /// Requested floating IP-address. If not set, a free one is selected.
    #[serde(default)]
    pub floating_ip: Option<Ipv4Addr>,
    pub internal_ip: Ipv4Addr,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct FloatingIpResp {
    pub uuid: Uuid,
    pub network_uuid: Uuid,
    pub floating_ip: Ipv4Addr,
    pub internal_ip: Ipv4Addr,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct FloatingIpInternalCreateReq {
    pub name: String,
    pub network_uuid: Uuid,
    pub floating_ip: Ipv4Addr,
    pub internal_ip: Ipv4Addr,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct FloatingIpInternalResp {
    pub uuid: Uuid,
    pub name: String,
    pub network_uuid: Uuid,
    pub floating_ip: Ipv4Addr,
    pub internal_ip: Ipv4Addr,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct FloatingIpBasicResp {
    pub uuid: Uuid,
    pub network_uuid: Uuid,
    pub floating_ip: Ipv4Addr,
    pub internal_ip: Ipv4Addr,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct FloatingIpListResp {
    pub floating_ips: Vec<FloatingIpBasicResp>,
}

#[derive(Debug, Deserialize, JsonSchema, ApiComponent)]
pub struct FloatingIpPath {
    /// The floating IPv4 address
    pub ip: Ipv4Addr,
}
