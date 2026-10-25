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
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

use ainari_common::secret::Secret;

/// State of the package of a backup, which is stored as one directory on a backup-target. The
/// package is only handled as a whole, so its state defines, which operations are allowed on its
/// files.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupStatus {
    /// The backup is created. Files can only be written into the package in this state.
    Creating,
    /// The package is complete and sealed, so it is read only from now on
    Ready,
    /// The package is deleted as a whole. A failed deletion stays in this state and can be repeated.
    Deleting,
    /// The package is incomplete or inconsistent and can only be deleted
    Error,
}

impl BackupStatus {
    /// Value of the status, like it is stored in the database
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupStatus::Creating => "CREATING",
            BackupStatus::Ready => "READY",
            BackupStatus::Deleting => "DELETING",
            BackupStatus::Error => "ERROR",
        }
    }
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BackupStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "CREATING" => Ok(BackupStatus::Creating),
            "READY" => Ok(BackupStatus::Ready),
            "DELETING" => Ok(BackupStatus::Deleting),
            "ERROR" => Ok(BackupStatus::Error),
            _ => Err(format!("'{value}' is not a valid backup-status")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupCreateReq {
    /// UUID of the backup, which is also the name of its directory on the backup-target
    pub uuid: Uuid,
    /// project, which the backup belongs to
    #[validate(length(min = 1, max = 255))]
    pub project_id: String,
    /// backup-target, where the backup is stored. Without it, ryokan selects one.
    pub host_uuid: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupListQuery {
    /// Project to list only its backups. Without it, all backups are listed, which are visible
    /// for the user.
    pub project_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupResp {
    pub uuid: Uuid,
    /// backup-target, which stores the package
    pub host_uuid: Uuid,
    /// address of the onsen of the backup-target
    pub onsen_address: String,
    /// path of the package relative to the storage-location of the onsen
    pub directory_path: String,
    pub backup_status: BackupStatus,
    /// number of files in the package, known after it was finalized
    pub file_count: u64,
    /// size of all files of the package in bytes, known after it was finalized
    pub total_size: u64,
    /// hex-encoded sha256 of the manifest of the package, known after it was finalized
    pub manifest_sha256: Option<String>,
    pub owner_id: String,
    pub project_id: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupListResp {
    pub backups: Vec<BackupResp>,
}

/// Maximum size in bytes of a single file, which is written into a package or read from it over
/// the api. It is meant for the metadata and the manifest of a backup, while large files, like
/// disks, are stored directly in the onsen.
pub const MAX_BACKUP_FILE_SIZE: usize = 16 * 1024 * 1024;

/// Maximum size in bytes of the json-message, which transfers a file of a package. The
/// base64-encoded content is a third larger than the file itself, plus some space for the other
/// fields of the message.
pub const MAX_BACKUP_FILE_MESSAGE_SIZE: usize = MAX_BACKUP_FILE_SIZE.div_ceil(3) * 4 + 4096;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupFileQuery {
    /// path of the file relative to the directory of the package
    #[validate(length(min = 1, max = 1024))]
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupFileWriteReq {
    /// path of the file relative to the directory of the package
    #[validate(length(min = 1, max = 1024))]
    pub path: String,
    /// base64-encoded content of the file
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupFileResp {
    /// path of the file relative to the directory of the package
    pub path: String,
    /// base64-encoded content of the file
    pub content: String,
}

/// Size and checksum of a file within the package of a backup.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct BackupFileInfoResp {
    /// path of the file relative to the directory of the package
    pub path: String,
    /// size of the file in bytes
    pub size: u64,
    /// hex-encoded sha256 of the content of the file
    pub sha256: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupDiskReq {
    /// virtual_machine, whose root-disk is stored in the package
    pub vm_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupDiskResp {
    /// address of the onsen of the backup-target, where the disk is uploaded to
    pub onsen_address: String,
    /// path of the disk relative to the storage-location of the onsen
    pub file_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupScanEntry {
    /// UUID of the backup, which is the name of its directory
    pub uuid: Uuid,
    /// true, if the package contains a manifest. A package without manifest is incomplete.
    pub has_manifest: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupScanResp {
    /// packages on the backup-target, which are not registered as backup
    pub backups: Vec<BackupScanEntry>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupManifestQuery {
    /// UUID of the package, which is the name of its directory on the backup-target
    pub backup_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupRegisterReq {
    /// UUID of the backup, which is the name of its directory on the backup-target
    pub uuid: Uuid,
    /// project, which the backup belongs to
    #[validate(length(min = 1, max = 255))]
    pub project_id: String,
    /// backup-target, where the package is stored
    pub host_uuid: Uuid,
}

/// Direction of a copy between the package of a backup and the normal onsens.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupCopyDirection {
    /// The file of an image is copied into the package, which must be in the state `CREATING`
    IntoBackup,
    /// A file of the package is copied to a normal onsen, where it can be registered as image.
    /// The package must be in the state `READY`.
    OutOfBackup,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupCopyReq {
    pub direction: BackupCopyDirection,
    /// path of the file relative to the directory of the package
    #[validate(length(min = 1, max = 1024))]
    pub path: String,
    /// `INTO_BACKUP`: image, whose file is copied into the package. `OUT_OF_BACKUP`: new UUID of
    /// the image, under which the file is stored on the normal onsen.
    pub image_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupCopyResp {
    /// address of the onsen, where the file was copied to
    pub onsen_address: String,
    /// path of the copy relative to the storage-location of the onsen
    pub file_path: String,
    /// size of the copied file in bytes
    pub size: u64,
}

// ==================================================================================================
// isekai
// ==================================================================================================

/// Minimum length of the passphrase of a backup, which protects the backup outside of the
/// deployment.
pub const MIN_BACKUP_PASSPHRASE_LENGTH: usize = 12;
/// Maximum length of the passphrase of a backup.
pub const MAX_BACKUP_PASSPHRASE_LENGTH: usize = 1024;

/// Content of a backup.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BackupKind {
    /// All resources of a project
    Project,
    /// A single virtual_machine with its root-disk, public-key, network and packet-filters
    VirtualMachine,
}

impl BackupKind {
    /// Value of the kind, like it is stored in the database
    pub fn as_str(&self) -> &'static str {
        match self {
            BackupKind::Project => "PROJECT",
            BackupKind::VirtualMachine => "VIRTUAL_MACHINE",
        }
    }
}

impl std::fmt::Display for BackupKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for BackupKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PROJECT" => Ok(BackupKind::Project),
            "VIRTUAL_MACHINE" => Ok(BackupKind::VirtualMachine),
            _ => Err(format!("'{value}' is not a valid backup-kind")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct ProjectBackupCreateReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    /// Passphrase, which encrypts the backup. It is required again for the restore and is never
    /// stored, so a backup can't be restored without it.
    #[validate(length(
        min = "MIN_BACKUP_PASSPHRASE_LENGTH",
        max = "MAX_BACKUP_PASSPHRASE_LENGTH"
    ))]
    pub passphrase: Secret,
    /// Shut the running virtual_machines down for the time of the saving of their root-disks and
    /// start them again afterwards, so the root-disks are consistent. Otherwise they are only
    /// paused shortly, which is like a power-loss for the applications within them.
    #[serde(default)]
    pub stop_vms: bool,
    /// Copy the images and snapshots of the project into the backup, so it is self-contained
    #[serde(default = "default_true")]
    pub include_images: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VirtualMachineBackupCreateReq {
    /// virtual_machine of the project of the user, which is saved
    pub virtual_machine_uuid: Uuid,
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    /// Passphrase, which encrypts the backup. It is required again for the restore and is never
    /// stored, so a backup can't be restored without it.
    #[validate(length(
        min = "MIN_BACKUP_PASSPHRASE_LENGTH",
        max = "MAX_BACKUP_PASSPHRASE_LENGTH"
    ))]
    pub passphrase: Secret,
    /// Shut the virtual_machine down for the time of the saving of its root-disk and start it
    /// again afterwards, so the root-disk is consistent
    #[serde(default)]
    pub stop_vm: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupRestoreReq {
    /// Passphrase, which was given, when the backup was created
    #[validate(length(min = 1, max = "MAX_BACKUP_PASSPHRASE_LENGTH"))]
    pub passphrase: Secret,
    /// Overwrite existing resources of the project with the same UUID. Without it, the restored
    /// resources get new UUIDs, if their UUIDs are already used.
    #[serde(default)]
    pub force: bool,
    /// Delete the resources of the project, which are not part of the backup
    #[serde(default)]
    pub prune: bool,
    /// Project, which the backup is restored into. Without it, the project of the backup is used.
    #[serde(default)]
    #[validate(length(min = 1, max = 255))]
    pub target_project_id: Option<String>,
    /// Start all restored virtual_machines. Without it, they get the power-state, which they had
    /// at the time of the backup.
    #[serde(default)]
    pub start_vms: bool,
    /// Create the network of a virtual_machine from the backup, if it doesn't exist in the
    /// project anymore
    #[serde(default = "default_true")]
    pub include_dependencies: bool,
    /// Only calculate and return the plan of the restore, without changing anything
    #[serde(default)]
    pub dry_run: bool,
}

/// Default for the flags, which are enabled, if they are not given
fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct IsekaiBackupResp {
    pub uuid: Uuid,
    pub name: String,
    pub kind: BackupKind,
    pub project_id: String,
    /// saved virtual_machine, only for backups of the kind `VIRTUAL_MACHINE`
    pub virtual_machine_uuid: Option<Uuid>,
    pub state: BackupStatus,
    /// version of ainari, which has created the backup
    pub ainari_version: String,
    /// backup-target, which stores the backup, if it is known
    pub host_uuid: Option<Uuid>,
    /// number of files of the backup, known after it was finished
    pub file_count: Option<u64>,
    /// size of all files of the backup in bytes, known after it was finished
    pub total_size: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct IsekaiBackupListResp {
    pub backups: Vec<IsekaiBackupResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobType {
    Backup,
    Restore,
}

impl JobType {
    /// Value of the type, like it is stored in the database
    pub fn as_str(&self) -> &'static str {
        match self {
            JobType::Backup => "BACKUP",
            JobType::Restore => "RESTORE",
        }
    }
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for JobType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "BACKUP" => Ok(JobType::Backup),
            "RESTORE" => Ok(JobType::Restore),
            _ => Err(format!("'{value}' is not a valid job-type")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobState {
    /// The job waits for its worker
    Queued,
    /// The job is processed right now
    Active,
    /// The job was processed successfully
    Finished,
    /// The job failed. The messages of the job contain the reason.
    Error,
}

impl JobState {
    /// Value of the state, like it is stored in the database
    pub fn as_str(&self) -> &'static str {
        match self {
            JobState::Queued => "QUEUED",
            JobState::Active => "ACTIVE",
            JobState::Finished => "FINISHED",
            JobState::Error => "ERROR",
        }
    }

    /// Checks, if the job has ended, successfully or not.
    pub fn is_done(&self) -> bool {
        matches!(self, JobState::Finished | JobState::Error)
    }
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for JobState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "QUEUED" => Ok(JobState::Queued),
            "ACTIVE" => Ok(JobState::Active),
            "FINISHED" => Ok(JobState::Finished),
            "ERROR" => Ok(JobState::Error),
            _ => Err(format!("'{value}' is not a valid job-state")),
        }
    }
}

/// Resource, which got a new UUID by a restore, because its original UUID was already used.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct IdMapEntryResp {
    /// kind of the resource, like `virtual_machine` or `network`
    pub resource_type: String,
    /// UUID of the resource within the backup
    pub old_uuid: Uuid,
    /// UUID of the restored resource
    pub new_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct JobResp {
    pub uuid: Uuid,
    pub job_type: JobType,
    pub backup_uuid: Uuid,
    /// project, which is saved or restored by the job
    pub project_id: String,
    pub state: JobState,
    /// step, which is processed right now, or the last step of a finished or failed job
    pub step: String,
    pub messages: Vec<String>,
    /// resources, which got a new UUID by a restore
    pub id_map: Vec<IdMapEntryResp>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct JobListResp {
    pub jobs: Vec<JobResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct JobListQuery {
    /// Backup to list only its jobs
    pub backup_uuid: Option<Uuid>,
}

/// Resource of the project, which is changed by a restore.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct PlannedResource {
    /// kind of the resource, like `virtual_machine` or `network`
    pub resource_type: String,
    /// UUID of the resource. For a packet-filter it is the UUID of its virtual_machine.
    pub uuid: Uuid,
    /// name of the resource. For a packet-filter it is its direction.
    pub name: String,
}

/// Usage of the quota of the project for one kind of resource after the restore.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct QuotaUsageResp {
    /// kind of the resource, like `virtual_machine` or `network`
    pub resource_type: String,
    /// number of resources of the project before the restore
    pub current: i64,
    /// number of resources of the project after the restore
    pub after_restore: i64,
    /// maximum number of resources of the project after the restore
    pub max: i64,
}

impl QuotaUsageResp {
    /// True, if the restore would exceed the quota. A restore, which reduces the number of
    /// resources, is always possible, also if the project is already above its quota.
    pub fn is_exceeded(&self) -> bool {
        self.after_restore > self.max && self.after_restore > self.current
    }
}

/// Plan of a restore, which shows, how the project is changed by it.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct RestorePlanResp {
    /// project, which the backup is restored into
    pub target_project_id: String,
    /// true, if the project doesn't exist and is created by the restore
    pub create_project: bool,
    /// resources, which get a new UUID, because their UUID is already used
    pub id_map: Vec<IdMapEntryResp>,
    /// existing resources, which are overwritten by the backup
    pub overwritten: Vec<PlannedResource>,
    /// resources of the project, which are not part of the backup and deleted by `prune`
    pub deleted: Vec<PlannedResource>,
    /// usage of the quota of the project after the restore
    pub quota: Vec<QuotaUsageResp>,
    /// hints about parts of the backup, which can't be restored completely
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct RestoreResp {
    /// plan of the restore
    pub plan: RestorePlanResp,
    /// job, which restores the backup and which can be polled. Empty for a dry-run.
    pub job: Option<JobResp>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupScanQuery {
    /// backup-target, which is scanned for packages, which are not known yet
    pub onsen_host_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct BackupImportReq {
    /// backup-target, which stores the package
    pub onsen_host_uuid: Uuid,
    /// UUID of the package, which is the name of its directory on the backup-target
    pub backup_uuid: Uuid,
    /// Passphrase, which was given, when the backup was created
    #[validate(length(min = 1, max = "MAX_BACKUP_PASSPHRASE_LENGTH"))]
    pub passphrase: Secret,
    /// Name of the backup. Without it, the backup is named after its UUID.
    #[serde(default)]
    #[validate(length(min = 4, max = 127))]
    pub name: Option<String>,
}

/// Response of a new backup, which is created by a job in the background.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct BackupJobResp {
    pub backup: IsekaiBackupResp,
    /// job, which creates the backup and which can be polled
    pub job: JobResp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_status_round_trip() {
        for status in [
            BackupStatus::Creating,
            BackupStatus::Ready,
            BackupStatus::Deleting,
            BackupStatus::Error,
        ] {
            assert_eq!(status.as_str().parse::<BackupStatus>(), Ok(status));
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, format!("\"{}\"", status.as_str()));
        }
        assert!("UNKNOWN".parse::<BackupStatus>().is_err());
    }

    #[test]
    fn test_max_backup_file_message_size() {
        use base64::Engine;
        use base64::engine::general_purpose::STANDARD as BASE64;

        // a file with the maximum size fits into the json-message
        let resp = BackupFileResp {
            path: "a".repeat(1024),
            content: BASE64.encode(vec![0u8; MAX_BACKUP_FILE_SIZE]),
        };
        let message = serde_json::to_string(&resp).unwrap();
        assert!(message.len() <= MAX_BACKUP_FILE_MESSAGE_SIZE);
    }

    #[test]
    fn test_isekai_enums_round_trip() {
        for kind in [BackupKind::Project, BackupKind::VirtualMachine] {
            assert_eq!(kind.as_str().parse::<BackupKind>(), Ok(kind));
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.as_str()));
        }
        for job_type in [JobType::Backup, JobType::Restore] {
            assert_eq!(job_type.as_str().parse::<JobType>(), Ok(job_type));
        }
        for state in [
            JobState::Queued,
            JobState::Active,
            JobState::Finished,
            JobState::Error,
        ] {
            assert_eq!(state.as_str().parse::<JobState>(), Ok(state));
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, format!("\"{}\"", state.as_str()));
        }
        assert!("UNKNOWN".parse::<BackupKind>().is_err());
        assert!("UNKNOWN".parse::<JobState>().is_err());
    }

    #[test]
    fn test_backup_requests() {
        let req: ProjectBackupCreateReq =
            serde_json::from_str(r#"{"name": "backup", "passphrase": "correct horse battery"}"#)
                .unwrap();
        assert!(req.validate().is_ok());
        assert!(req.include_images);
        assert!(!req.stop_vms);

        // a short passphrase is rejected
        let req: ProjectBackupCreateReq =
            serde_json::from_str(r#"{"name": "backup", "passphrase": "short"}"#).unwrap();
        assert!(req.validate().is_err());

        let req: BackupRestoreReq =
            serde_json::from_str(r#"{"passphrase": "correct horse battery"}"#).unwrap();
        assert!(req.validate().is_ok());
        assert!(req.include_dependencies);
        assert!(!req.force && !req.prune && !req.start_vms && !req.dry_run);
        assert!(req.target_project_id.is_none());
    }

    #[test]
    fn test_quota_usage_is_exceeded() {
        let usage = |current, after_restore, max| QuotaUsageResp {
            resource_type: "network".to_string(),
            current,
            after_restore,
            max,
        };
        assert!(!usage(1, 3, 3).is_exceeded());
        assert!(usage(1, 4, 3).is_exceeded());
        // a project above its quota can still be reduced
        assert!(!usage(6, 5, 3).is_exceeded());
        assert!(!usage(5, 5, 3).is_exceeded());
    }

    #[test]
    fn test_backup_copy_direction() {
        let json = serde_json::to_string(&BackupCopyDirection::OutOfBackup).unwrap();
        assert_eq!(json, "\"OUT_OF_BACKUP\"");
        let direction: BackupCopyDirection = serde_json::from_str("\"INTO_BACKUP\"").unwrap();
        assert_eq!(direction, BackupCopyDirection::IntoBackup);
    }
}
