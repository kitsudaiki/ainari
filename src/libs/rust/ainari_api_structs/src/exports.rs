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

use crate::network_filter_structs::{FilterDirection, IpRangeRule, PortRangeRule};
use ainari_common::secret::Secret;

/// Quota of a project, like it is stored in a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct QuotaExport {
    #[validate(range(min = 0))]
    pub max_virtual_machine: i32,
    #[validate(range(min = 0))]
    pub max_image: i32,
    #[validate(range(min = 0))]
    pub max_secret: i32,
    #[validate(range(min = 0))]
    pub max_network: i32,
    #[validate(range(min = 0))]
    pub max_floating_ip: i32,
}

/// Project with its quota, like it is exported from miko into a backup. Users and their
/// assignments to the project are not part of it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct MikoProjectExport {
    /// ID of the project at the time of the backup
    pub id: String,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    #[validate(nested)]
    pub quota: QuotaExport,
}

/// Image or snapshot, like it is exported from ryokan into a backup. The location of its file is
/// not part of it, because the file is stored within the backup itself.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct ImageExport {
    /// UUID of the image at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    /// type of the unencrypted file (e.g. "qcow2")
    #[validate(length(min = 1, max = 31))]
    pub file_type: String,
    /// size of the unencrypted file in bytes
    pub file_size: u64,
    pub is_snapshot: bool,
    /// secret, which encrypts the file of the image
    pub secret_uuid: Uuid,
}

/// Images and snapshots of a project, like they are exported from ryokan into a backup.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RyokanProjectExport {
    #[validate(nested)]
    pub images: Vec<ImageExport>,
}

/// Network, like it is exported from hanami into a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct NetworkExport {
    /// UUID of the network at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 63))]
    pub subnet: String,
    pub disable_encryption: bool,
}

/// Virtual_machine, like it is exported from hanami into a backup. The disk and the config within
/// the virtual_machine are exported by sakura.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct HanamiVirtualMachineExport {
    /// UUID of the virtual_machine at the time of the backup
    pub uuid: Uuid,
    pub name: String,
    pub number_of_cores: i64,
    /// memory in MiB
    pub memory_size: i64,
    /// size of the root-disk in GiB
    pub disk_size: i64,
    pub network_uuid: Uuid,
    pub internal_ip: Ipv4Addr,
    pub mac_address: String,
}

/// Packet filter of one direction of a virtual_machine, like it is exported from hanami into a
/// backup.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct NetworkFilterExport {
    /// virtual_machine at the time of the backup, which the filter belongs to
    pub virtual_machine_uuid: Uuid,
    pub direction: FilterDirection,
    /// include-list of the ip-ranges
    #[schemars(with = "Vec<String>")]
    pub ip_ranges: Vec<IpRangeRule>,
    /// include-list of the ports
    #[schemars(with = "Vec<String>")]
    pub ports: Vec<PortRangeRule>,
}

/// Floating ip-address, like it is exported from hanami into a backup. It is allocated again from
/// the pool of the target on restore, so the address is only a preference.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct FloatingIpExport {
    /// UUID of the floating ip-address at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub floating_ip: Ipv4Addr,
    /// virtual_machine, which the floating ip-address was attached to
    pub virtual_machine_uuid: Option<Uuid>,
}

/// Networks, virtual_machines, packet filters and floating ip-addresses of a project, like they
/// are exported from hanami into a backup.
#[derive(Serialize, Deserialize, Debug, Clone, Default, JsonSchema, ApiComponent)]
pub struct HanamiProjectExport {
    pub networks: Vec<NetworkExport>,
    pub virtual_machines: Vec<HanamiVirtualMachineExport>,
    pub network_filters: Vec<NetworkFilterExport>,
    pub floating_ips: Vec<FloatingIpExport>,
}

/// Secret, like it is exported from omamori into a backup. The payload never leaves the omamori in
/// plain text, but is wrapped with the wrap-key of the backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct SecretExport {
    /// UUID of the secret at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    /// kind of owner of the secret, like `user` or `image`
    #[validate(length(min = 1, max = 63))]
    pub owned_by: String,
    /// resource, which owns the secret, if it is not owned by a user
    pub resource_uuid: Option<Uuid>,
    /// base64-encoded payload, which is wrapped with the wrap-key (AES-256-GCM with the UUID of the
    /// secret as additional data)
    pub wrapped_payload: String,
}

/// Public-key, like it is exported from omamori into a backup. The fingerprint is not part of it,
/// because it is calculated from the key again, when it is imported.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct PublicKeyExport {
    /// UUID of the public-key at the time of the backup
    pub uuid: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    /// the ssh-public-key in its one-line openssh-representation
    #[validate(length(min = 8, max = 4096))]
    pub public_key: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct OmamoriProjectExportReq {
    /// base64-encoded key with 256 bit, which wraps the payloads of the exported secrets. It is
    /// never stored or logged by the omamori.
    pub wrap_key: Secret,
    /// backup, whose own secrets are exported too. The secrets of all other backups are never
    /// exported.
    pub backup_uuid: Option<Uuid>,
}

/// Secrets and public-keys of a project, like they are exported from omamori into a backup.
#[derive(Serialize, Deserialize, Debug, Clone, Default, JsonSchema, ApiComponent, Validate)]
pub struct OmamoriProjectExport {
    #[validate(nested)]
    pub secrets: Vec<SecretExport>,
    #[validate(nested)]
    pub public_keys: Vec<PublicKeyExport>,
}

/// Config of a virtual_machine, like it is exported from sakura into a backup. The root-disk is
/// saved separately into the backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct SakuraVirtualMachineExport {
    /// UUID of the virtual_machine at the time of the backup
    pub uuid: Uuid,
    pub name: String,
    /// Power-state of the virtual_machine: RESERVED, CREATED, RUNNING, STOPPED, RESTORING or ERROR
    pub vm_state: String,
    pub number_of_cores: i32,
    /// memory of the virtual_machine in bytes
    pub memory_size: i64,
    /// size of the root-disk in GiB
    pub disk_size: i64,
    /// image, which the virtual_machine was created from
    pub image_uuid: Uuid,
    pub public_key_uuid: Uuid,
    pub network_uuid: Uuid,
    pub internal_ip: Ipv4Addr,
    pub mac_address: String,
}
