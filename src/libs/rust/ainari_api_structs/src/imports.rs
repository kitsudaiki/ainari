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

use std::collections::BTreeMap;

use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::exports::{
    FloatingIpExport, ImageExport, MikoProjectExport, NetworkExport, NetworkFilterExport,
    PublicKeyExport, SecretExport,
};
use ainari_common::secret::Secret;

/// Resource, which was imported from a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct ImportedResource {
    /// UUID of the resource within the backup
    pub source_uuid: Uuid,
    /// UUID of the imported resource, which differs, if it was changed by the `id_map`
    pub uuid: Uuid,
    /// true, if an existing resource was overwritten
    pub overwritten: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MikoProjectImportReq {
    /// project from the backup. Its ID is ignored, the project is imported under the ID of the
    /// path, so a backup can also be restored into another project.
    #[validate(nested)]
    pub project: MikoProjectExport,
    /// overwrite name and quota of an already existing project
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct MikoProjectImportResp {
    pub project_id: String,
    /// true, if the project didn't exist before and was created by the import
    pub created: bool,
    /// true, if the quota of the backup was applied. The quota of an existing project is only
    /// overwritten for an admin, so a user can't raise the quota of the own project with a backup.
    pub quota_imported: bool,
}

/// Image from a backup, whose file was already copied to a normal onsen.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RyokanImageImport {
    #[validate(nested)]
    pub image: ImageExport,
    /// address of the onsen, where the file of the image was copied to
    #[validate(length(min = 1, max = 255))]
    pub onsen_address: String,
    /// path of the file on the onsen, which must be within `images/`
    #[validate(length(min = 1, max = 1024))]
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct RyokanProjectImportReq {
    #[validate(nested)]
    pub images: Vec<RyokanImageImport>,
    /// overwrite existing images of the project with the same UUID
    #[serde(default)]
    pub force: bool,
    /// new UUIDs of the resources, whose UUID was already taken. It is applied to the UUIDs of the
    /// images and to their secrets. UUIDs, which are not part of it, are kept.
    #[serde(default)]
    pub id_map: BTreeMap<Uuid, Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct RyokanProjectImportResp {
    pub project_id: String,
    pub images: Vec<ImportedResource>,
}

/// Import of the resources of a project from a backup into hanami. The virtual_machines are not
/// imported here, but reserved like new ones, so the networks are imported before them, the
/// packet filters and floating ip-addresses afterwards with a second request.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct HanamiProjectImportReq {
    #[serde(default)]
    #[validate(nested)]
    pub networks: Vec<NetworkExport>,
    #[serde(default)]
    #[validate(nested)]
    pub network_filters: Vec<NetworkFilterExport>,
    #[serde(default)]
    #[validate(nested)]
    pub floating_ips: Vec<FloatingIpExport>,
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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct OmamoriProjectImportReq {
    /// base64-encoded key with 256 bit, which was used to wrap the payloads of the secrets. It is
    /// never stored or logged by the omamori.
    pub wrap_key: Secret,
    #[validate(nested)]
    pub secrets: Vec<SecretExport>,
    #[validate(nested)]
    pub public_keys: Vec<PublicKeyExport>,
    /// overwrite existing secrets and public-keys of the project with the same UUID
    #[serde(default)]
    pub force: bool,
    /// new UUIDs of the resources, whose UUID was already taken. It is applied to the UUIDs of
    /// the secrets and public-keys and to the resources, which own the secrets. UUIDs, which are
    /// not part of it, are kept.
    #[serde(default)]
    pub id_map: BTreeMap<Uuid, Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct OmamoriProjectImportResp {
    pub project_id: String,
    pub secrets: Vec<ImportedResource>,
    pub public_keys: Vec<ImportedResource>,
}
