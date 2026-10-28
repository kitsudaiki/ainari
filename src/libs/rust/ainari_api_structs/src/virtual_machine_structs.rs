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
    /// vm-type, which defines the number of cores and the memory of the virtual_machine
    pub vm_type_uuid: Uuid,
    /// size of the disk of the virtual_machine in GiB
    #[validate(range(min = 1))]
    pub disk_size: i64,
    pub network_uuid: Uuid,
    /// Place the virtual_machine on a host, which is isolated for the project of the user
    #[serde(default)]
    pub isolated_host: bool,
    /// Sakura-host, which has to run the virtual_machine, instead of a randomly selected one.
    /// Only admins are allowed to set it.
    #[serde(default)]
    pub host_uuid: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VirtualMachineInternalCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    pub network_uuid: Uuid,
    pub number_of_cores: i32,
    /// memory of the virtual_machine in bytes
    pub memory_size: i64,
    /// size of the disk of the virtual_machine in GiB
    #[validate(range(min = 1))]
    pub disk_size: i64,
    pub internal_ip: Ipv4Addr,
    pub tap_name: String,
    pub mac_address: String,
    /// UUID of the virtual_machine, for example the original one of a backup. Without it, a new
    /// UUID is generated.
    #[serde(default)]
    pub uuid: Option<Uuid>,
}

/// Reservation of a virtual_machine by another component, for example to restore it from a
/// backup. Unlike `VirtualMachineCreateReq` the resources are given directly instead of a vm-type,
/// and the UUID and the addresses can be predefined.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VirtualMachineReserveInternalReq {
    /// UUID of the virtual_machine. Without it, a new UUID is generated.
    #[serde(default)]
    pub uuid: Option<Uuid>,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    pub network_uuid: Uuid,
    #[validate(range(min = 1))]
    pub number_of_cores: i32,
    /// memory of the virtual_machine in MiB
    #[validate(range(min = 1))]
    pub memory_size: i64,
    /// size of the disk of the virtual_machine in GiB
    #[validate(range(min = 1))]
    pub disk_size: i64,
    /// internal IP-address within the network. Without it, the next free one is used.
    #[serde(default)]
    pub internal_ip: Option<Ipv4Addr>,
    /// MAC-address of the virtual_machine. Without it, the next free one is used.
    #[serde(default)]
    pub mac_address: Option<String>,
    /// Place the virtual_machine on a host, which is isolated for the project
    #[serde(default)]
    pub isolated_host: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VirtualMachineResp {
    pub uuid: Uuid,
    pub name: String,
    /// Power-state of the virtual_machine: RESERVED, CREATED, RUNNING, STOPPED, RESTORING or ERROR
    pub vm_state: String,
    pub number_of_cores: i32,
    /// memory of the virtual_machine; in bytes from sakura, in MiB from hanami
    pub memory_size: i64,
    /// size of the disk of the virtual_machine in GiB
    pub disk_size: i64,
    pub image_uuid: Uuid,
    pub network_uuid: Uuid,
    pub internal_ip: Ipv4Addr,
    /// Address of the sakura-host of the virtual_machine, which is a proxy of the torii at the
    /// edge, so it stays the same, when the virtual_machine is moved to another sakura-host.
    /// Empty in the responses of sakura itself.
    pub torii_address: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

/// Sakura-host, which runs a virtual_machine.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct VirtualMachineHostResp {
    pub virtual_machine_uuid: Uuid,
    pub host_uuid: Uuid,
    /// Internal address of the sakura-host, where its internal endpoints are reachable
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VirtualMachineBasicResp {
    pub uuid: Uuid,
    pub name: String,
    /// Address of the sakura-host of the virtual_machine, like `torii_address`
    pub proxy_address: String,
    pub number_of_cores: i32,
    /// memory of the virtual_machine; in bytes from sakura, in MiB from hanami
    pub memory_size: i64,
    /// size of the disk of the virtual_machine in GiB
    pub disk_size: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VirtualMachineListResp {
    pub virtual_machines: Vec<VirtualMachineBasicResp>,
}
