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
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
use validator::Validate;

use crate::floating_ip_structs::HanamiFloatingIpExport;
use crate::network_filter_structs::HanamiNetworkFilterExport;
use crate::project_structs::ImportedResource;
use crate::virtual_machine_structs::HanamiVirtualMachineExport;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct NetworkCreateReq {
    pub name: String,
    pub subnet: String,
    /// Don't encrypt the traffic between the virtual machines of the network, which run on
    /// different hosts. The network gets neither IPsec-encryption nor a MLS-group then.
    #[serde(default)]
    pub disable_encryption: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct NetworkResp {
    pub uuid: Uuid,
    pub name: String,
    pub subnet: String,
    pub disable_encryption: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct NetworkBasicResp {
    pub uuid: Uuid,
    pub name: String,
    pub subnet: String,
    pub disable_encryption: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct NetworkListResp {
    pub networks: Vec<NetworkBasicResp>,
}

/// Network, like it is exported from hanami into a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct HanamiNetworkExport {
    /// UUID of the network at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 63))]
    pub subnet: String,
    pub disable_encryption: bool,
}

/// Networks, virtual_machines, packet filters and floating ip-addresses of a project, like they
/// are exported from hanami into a backup.
#[derive(Serialize, Deserialize, Debug, Clone, Default, JsonSchema, ApiComponent)]
pub struct HanamiProjectExport {
    pub networks: Vec<HanamiNetworkExport>,
    pub virtual_machines: Vec<HanamiVirtualMachineExport>,
    pub network_filters: Vec<HanamiNetworkFilterExport>,
    pub floating_ips: Vec<HanamiFloatingIpExport>,
}

/// Import of the resources of a project from a backup into hanami. The virtual_machines are not
/// imported here, but reserved like new ones, so the networks are imported before them, the
/// packet filters and floating ip-addresses afterwards with a second request.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct HanamiProjectImportReq {
    #[serde(default)]
    #[validate(nested)]
    pub networks: Vec<HanamiNetworkExport>,
    #[serde(default)]
    #[validate(nested)]
    pub network_filters: Vec<HanamiNetworkFilterExport>,
    #[serde(default)]
    #[validate(nested)]
    pub floating_ips: Vec<HanamiFloatingIpExport>,
    /// overwrite existing resources of the project
    #[serde(default)]
    pub force: bool,
    /// new UUIDs of the resources, whose UUID was already taken. It is applied to the networks
    /// and to the virtual_machines, which the packet filters and floating ip-addresses belong
    /// to. UUIDs, which are not part of it, are kept.
    #[serde(default)]
    pub id_map: BTreeMap<Uuid, Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct HanamiProjectImportResp {
    pub project_id: String,
    pub networks: Vec<ImportedResource>,
    /// the floating ip-addresses get new UUIDs, because they are allocated again
    pub floating_ips: Vec<ImportedResource>,
}
