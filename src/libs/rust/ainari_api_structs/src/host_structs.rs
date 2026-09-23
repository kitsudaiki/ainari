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
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use ainari_common::secret::Secret;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct HostCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(length(min = 4, max = 127))]
    pub host_address: String,
    pub deleted_uuids: UuidList,
    #[validate(length(min = 4, max = 127))]
    pub registration_key: Secret,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct SakuraHostCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(length(min = 4, max = 127))]
    pub host_address: String,
    pub deleted_uuids: UuidList,
    #[validate(length(min = 4, max = 127))]
    pub registration_key: Secret,
    /// number of cpu-threads of the host
    pub number_of_cores: u64,
    /// total memory of the host in MiB
    pub memory_size: u64,
    /// total size of the disk, which holds the virtual-machines, in GiB
    pub disk_space: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Default)]
pub struct UuidList {
    pub list: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct HostResp {
    pub uuid: Uuid,
    pub name: String,
    pub host_address: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct HostBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub host_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct HostListResp {
    pub hosts: Vec<HostBasicResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SakuraHostBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub host_address: String,
    /// number of cpu-threads of the host
    pub number_of_cores: u64,
    /// number of cpu-threads, which are allocated by virtual-machines
    pub used_number_of_cores: u64,
    /// total memory of the host in MiB
    pub memory_size: u64,
    /// memory in MiB, which is allocated by virtual-machines
    pub amount_of_used_memory: u64,
    /// total size of the disk, which holds the virtual-machines, in GiB
    pub disk_space: u64,
    /// disk-space in GiB, which is allocated by virtual-machines
    pub amount_of_used_disk_space: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct SakuraHostListResp {
    pub hosts: Vec<SakuraHostBasicResp>,
}
