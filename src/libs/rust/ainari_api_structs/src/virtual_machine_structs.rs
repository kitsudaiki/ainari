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
pub struct VirtualMachineCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    pub number_of_cores: i32,
    pub memory_size: i64,
    pub network_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VirtualMachineInternalCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    pub network_uuid: Uuid,
    pub number_of_cores: i32,
    pub memory_size: i64,
    pub internal_ip: Ipv4Addr,
    pub tap_name: String,
    pub mac_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VirtualMachineResp {
    pub uuid: Uuid,
    pub name: String,
    pub is_created: bool,
    pub number_of_cores: i32,
    pub memory_size: i64,
    pub image_uuid: Uuid,
    pub network_uuid: Uuid,
    pub internal_ip: Ipv4Addr,
    pub torii_port: u16,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VirtualMachineBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub proxy_port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VirtualMachineListResp {
    pub virtual_machines: Vec<VirtualMachineBasicResp>,
}
