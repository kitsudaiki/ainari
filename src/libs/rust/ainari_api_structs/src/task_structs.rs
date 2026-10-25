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
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

#[cfg(feature = "diesel")]
use diesel::{AsExpression, FromSqlRow};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub enum TaskResourceType {
    VirtualMachine = 0,
    Image = 1,
    Volume = 2,
}

impl fmt::Display for TaskResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskResourceType::VirtualMachine => "VirtualMachine",
            TaskResourceType::Image => "Image",
            TaskResourceType::Volume => "Volume",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TaskResourceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VirtualMachine" => Ok(TaskResourceType::VirtualMachine),
            "Image" => Ok(TaskResourceType::Image),
            "Volume" => Ok(TaskResourceType::Volume),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Varchar))]
pub enum TaskType {
    VirtualMachineCreate = 0,
    VirtualMachineDelete = 1,
    SnapshotSave = 2,
    SnapshotRestore = 3,
    VirtualMachineStart = 4,
    VirtualMachineStop = 5,
    VirtualMachineReboot = 6,
    MigrationPrepare = 7,
    MigrationImport = 8,
    MigrationRemove = 9,
    MigrationCancel = 10,
    /// saves the root-disk of a virtual_machine into a backup of isekai
    BackupSave = 11,
    /// replaces the root-disk of a virtual_machine with the one of a backup of isekai
    BackupRestore = 12,
}

#[cfg(feature = "diesel")]
impl diesel::deserialize::FromSql<diesel::sql_types::Varchar, diesel::sqlite::Sqlite> for TaskType {
    fn from_sql(
        bytes: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let task_type_str = <String as diesel::deserialize::FromSql<
            diesel::sql_types::Varchar,
            diesel::sqlite::Sqlite,
        >>::from_sql(bytes)?;

        std::str::FromStr::from_str(&task_type_str).map_err(|_| {
            // Diesel expects errors to be boxed, so we cast a String error using .into()
            format!("Unrecognized task type in database: {}", task_type_str).into()
        })
    }
}

#[cfg(feature = "diesel")]
impl diesel::serialize::ToSql<diesel::sql_types::Varchar, diesel::sqlite::Sqlite> for TaskType {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        let task_type_str = self.to_string();
        out.set_value(task_type_str);

        // Tell Diesel that this value is NOT null
        Ok(diesel::serialize::IsNull::No)
    }
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskType::VirtualMachineCreate => "VirtualMachineCreateTask",
            TaskType::VirtualMachineDelete => "VirtualMachineDeleteTask",
            TaskType::SnapshotSave => "SnapshotSaveTask",
            TaskType::SnapshotRestore => "SnapshotRestoreTask",
            TaskType::VirtualMachineStart => "VirtualMachineStartTask",
            TaskType::VirtualMachineStop => "VirtualMachineStopTask",
            TaskType::VirtualMachineReboot => "VirtualMachineRebootTask",
            TaskType::MigrationPrepare => "MigrationPrepareTask",
            TaskType::MigrationImport => "MigrationImportTask",
            TaskType::MigrationRemove => "MigrationRemoveTask",
            TaskType::MigrationCancel => "MigrationCancelTask",
            TaskType::BackupSave => "BackupSaveTask",
            TaskType::BackupRestore => "BackupRestoreTask",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TaskType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VirtualMachineCreateTask" => Ok(TaskType::VirtualMachineCreate),
            "VirtualMachineDeleteTask" => Ok(TaskType::VirtualMachineDelete),
            "SnapshotSaveTask" => Ok(TaskType::SnapshotSave),
            "SnapshotRestoreTask" => Ok(TaskType::SnapshotRestore),
            "VirtualMachineStartTask" => Ok(TaskType::VirtualMachineStart),
            "VirtualMachineStopTask" => Ok(TaskType::VirtualMachineStop),
            "VirtualMachineRebootTask" => Ok(TaskType::VirtualMachineReboot),
            "MigrationPrepareTask" => Ok(TaskType::MigrationPrepare),
            "MigrationImportTask" => Ok(TaskType::MigrationImport),
            "MigrationRemoveTask" => Ok(TaskType::MigrationRemove),
            "MigrationCancelTask" => Ok(TaskType::MigrationCancel),
            "BackupSaveTask" => Ok(TaskType::BackupSave),
            "BackupRestoreTask" => Ok(TaskType::BackupRestore),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, JsonSchema, ApiComponent)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Varchar))]
pub enum TaskState {
    Created = 0,
    Queued = 1,
    Active = 2,
    Aborted = 3,
    Finished = 4,
    Error = 5,
}

#[cfg(feature = "diesel")]
impl diesel::deserialize::FromSql<diesel::sql_types::Varchar, diesel::sqlite::Sqlite>
    for TaskState
{
    fn from_sql(
        bytes: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        let task_state_str = <String as diesel::deserialize::FromSql<
            diesel::sql_types::Varchar,
            diesel::sqlite::Sqlite,
        >>::from_sql(bytes)?;

        std::str::FromStr::from_str(&task_state_str).map_err(|_| {
            // Diesel expects errors to be boxed, so we cast a String error using .into()
            format!("Unrecognized task state in database: {}", task_state_str).into()
        })
    }
}

#[cfg(feature = "diesel")]
impl diesel::serialize::ToSql<diesel::sql_types::Varchar, diesel::sqlite::Sqlite> for TaskState {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        let task_state_str = self.to_string();
        out.set_value(task_state_str);

        // Tell Diesel that this value is NOT null
        Ok(diesel::serialize::IsNull::No)
    }
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            TaskState::Created => "CREATED",
            TaskState::Queued => "QUEUED",
            TaskState::Active => "ACTIVE",
            TaskState::Aborted => "ABORTED",
            TaskState::Finished => "FINISHED",
            TaskState::Error => "ERROR",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TaskState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CREATED" => Ok(TaskState::Created),
            "QUEUED" => Ok(TaskState::Queued),
            "ACTIVE" => Ok(TaskState::Active),
            "ABORTED" => Ok(TaskState::Aborted),
            "FINISHED" => Ok(TaskState::Finished),
            "ERROR" => Ok(TaskState::Error),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VirtualMachineCreateTaskReq {
    pub vm_uuid: Uuid,
    pub image_uuid: Uuid,
    pub public_key_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskSnapshotSaveReq {
    #[validate(length(min = 4, max = 127))]
    pub name: String,
    /// Already existing secret in the omamori, which is used to encrypt the snapshot. Without it,
    /// a new secret is generated for the snapshot.
    #[serde(default)]
    pub secret_uuid: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskBackupSaveReq {
    /// Backup in the state CREATING, which gets the root-disk as one of its disks
    pub backup_uuid: Uuid,
    /// Secret in the omamori, which is used to encrypt the root-disk
    pub secret_uuid: Uuid,
    /// Shut a running virtual_machine down before the root-disk is saved and start it again
    /// afterwards, so the saved root-disk is consistent
    #[serde(default)]
    pub stop_vm: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskBackupRestoreReq {
    /// Backup, whose root-disk is restored
    pub backup_uuid: Uuid,
    /// Snapshot-image with the root-disk of the backup, which was copied out of the backup
    pub image_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct TaskSnapshotRestoreReq {
    /// Image, which must be a snapshot, to reset the root-disk of the virtual_machine to
    pub image_uuid: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskResp {
    pub uuid: Uuid,
    pub description: String,
    pub task_type: TaskType,
    pub state: TaskState,
    pub queued_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub messages: Vec<String>,
    pub created_by: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskBasicResp {
    pub uuid: Uuid,
    pub description: String,
    pub task_type: TaskType,
    pub state: TaskState,
    pub queued_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskListQuery {
    /// Resource, like a virtual_machine, to list only its tasks. Without it, all tasks of the
    /// sakura-host are listed.
    pub resource_uuid: Option<Uuid>,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct TaskListResp {
    pub tasks: Vec<TaskBasicResp>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_round_trip() {
        // the names are stored in the database of sakura, so they must be parsed again
        for task_type in [
            TaskType::VirtualMachineCreate,
            TaskType::VirtualMachineDelete,
            TaskType::SnapshotSave,
            TaskType::SnapshotRestore,
            TaskType::VirtualMachineStart,
            TaskType::VirtualMachineStop,
            TaskType::VirtualMachineReboot,
            TaskType::MigrationPrepare,
            TaskType::MigrationImport,
            TaskType::MigrationRemove,
            TaskType::MigrationCancel,
            TaskType::BackupSave,
            TaskType::BackupRestore,
        ] {
            assert_eq!(
                task_type.to_string().parse::<TaskType>(),
                Ok(task_type.clone())
            );
        }
        assert_eq!(TaskType::BackupSave.to_string(), "BackupSaveTask");
        assert_eq!(
            serde_json::to_string(&TaskType::BackupRestore).unwrap(),
            "\"BackupRestore\""
        );
    }
}
