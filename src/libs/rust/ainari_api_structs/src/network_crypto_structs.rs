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

use ainari_common::secret::Secret;
use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoKeyReq {
    pub direction: String,
    pub local_ip: Ipv4Addr,
    pub remote_ip: Ipv4Addr,
    pub peer_gateway_ip: Ipv4Addr,
    pub spi: u32,
    pub key: Secret,
}

#[derive(Debug, Default, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoKeyListResp {
    pub keys: Vec<CryptoKeyResp>,
}

#[derive(Debug, Deserialize, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
pub struct CryptoKeyResp {
    pub direction: String,
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

#[derive(Debug, Default, Serialize, Clone, JsonSchema, ApiComponent, Validate)]
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
