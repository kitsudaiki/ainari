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
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::public_key_structs::OmamoriPublicKeyExport;
use crate::secret_structs::OmamoriSecretExport;
use ainari_common::enums::ProjectRole;
use ainari_common::secret::Secret;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub id: String,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectResp {
    pub id: String,
    pub name: String,
    /// lock of the project during a backup or restore, None if the project is not locked
    #[serde(default)]
    pub lock: Option<ProjectLock>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectBasicResp {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectListResp {
    pub projects: Vec<ProjectBasicResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectInvitedResp {
    pub project_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectInvitedListResp {
    pub projects: Vec<ProjectInvitedResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectMemberResp {
    pub user_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectMemberListResp {
    pub members: Vec<ProjectMemberResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectAddUserReq {
    #[validate(length(min = 4, max = 127))]
    pub user_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectAddUserResp {
    pub user_id: String,
    pub project_id: String,
    pub project_role: ProjectRole,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectRemoveUserReq {
    #[validate(length(min = 4, max = 127))]
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectResourceCountInternalResp {
    /// number of the existing resources of the project, by the type of the resource
    pub resource_counts: BTreeMap<String, i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectLockReq {
    /// job, which locks the project. Only this job can remove the lock again.
    pub job_uuid: Uuid,
    /// description, why the project is locked, like the backup or restore, which is running
    #[validate(length(min = 1, max = 4096))]
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectLockAdminReq {
    /// description, why the admin has locked the project
    #[validate(length(min = 1, max = 4096))]
    pub reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectUnlockReq {
    /// job, which has locked the project
    pub job_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct ProjectLock {
    /// job, which has locked the project
    pub locked_by_job: Uuid,
    /// description, why the project is locked
    pub lock_reason: String,
    pub locked_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ProjectLockResp {
    pub project_id: String,
    /// lock of the project, None if the project is not locked
    pub lock: Option<ProjectLock>,
}

/// Quota of a project, like it is stored in a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent, Validate)]
pub struct MikoQuotaExport {
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
    pub quota: MikoQuotaExport,
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
    pub secrets: Vec<OmamoriSecretExport>,
    #[validate(nested)]
    pub public_keys: Vec<OmamoriPublicKeyExport>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct OmamoriProjectImportReq {
    /// base64-encoded key with 256 bit, which was used to wrap the payloads of the secrets. It is
    /// never stored or logged by the omamori.
    pub wrap_key: Secret,
    #[validate(nested)]
    pub secrets: Vec<OmamoriSecretExport>,
    #[validate(nested)]
    pub public_keys: Vec<OmamoriPublicKeyExport>,
    /// overwrite existing secrets and public-keys of the project with the same UUID
    #[serde(default)]
    pub force: bool,
    /// new UUIDs of the resources, whose UUID was already taken. It is applied to the UUIDs of
    /// the secrets and public-keys and to the resources, which own the secrets. UUIDs, which are
    /// not part of it, are kept.
    #[serde(default)]
    pub id_map: BTreeMap<Uuid, Uuid>,
}

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

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct OmamoriProjectImportResp {
    pub project_id: String,
    pub secrets: Vec<ImportedResource>,
    pub public_keys: Vec<ImportedResource>,
}
