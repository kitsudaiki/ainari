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

//! Job-steps of a backup.
//!
//! The project is locked by the endpoint, before the job is queued, so the resources of the
//! project don't change, while the root-disks are saved and the metadata is collected.

mod backup_files;
mod collect_metadata;
mod run_backup;
mod save_virtual_machines;

pub use backup_files::delete_backup_content;
pub use run_backup::run_backup;

use serde::Serialize;
use uuid::Uuid;

use ainari_api_structs::backup_structs::BackupKind;
use ainari_common::secret::Secret;

/// Resources, which are saved by a backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupScope {
    /// All resources of the project
    Project,
    /// A single virtual_machine with its root-disk, public-key, network and packet-filters
    VirtualMachine(Uuid),
}

impl BackupScope {
    /// Kind of the backup, which is created for the scope.
    pub fn kind(&self) -> BackupKind {
        match self {
            BackupScope::Project => BackupKind::Project,
            BackupScope::VirtualMachine(_) => BackupKind::VirtualMachine,
        }
    }

    /// Saved virtual_machine of a backup of a single virtual_machine.
    pub fn virtual_machine_uuid(&self) -> Option<Uuid> {
        match self {
            BackupScope::Project => None,
            BackupScope::VirtualMachine(uuid) => Some(*uuid),
        }
    }
}

/// Backup, which is created by a job.
#[derive(Debug)]
pub struct BackupJob {
    pub scope: BackupScope,
    /// Passphrase, which encrypts the backup. It only exists in the memory of the job.
    pub passphrase: Secret,
    /// Shut the running virtual_machines down, while their root-disks are saved
    pub stop_vms: bool,
    /// Copy the files of the images of the project into the backup
    pub include_images: bool,
}

/// Options of a backup-job, like they are stored in the database. The passphrase is never part
/// of it.
#[derive(Serialize)]
pub struct BackupJobOptions {
    pub kind: BackupKind,
    pub virtual_machine_uuid: Option<Uuid>,
    pub stop_vms: bool,
    pub include_images: bool,
}

impl BackupJob {
    /// Options of the job, which are stored in the database.
    pub fn options(&self) -> String {
        serde_json::to_string(&BackupJobOptions {
            kind: self.scope.kind(),
            virtual_machine_uuid: self.scope.virtual_machine_uuid(),
            stop_vms: self.stop_vms,
            include_images: self.include_images,
        })
        .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_contain_no_passphrase() {
        let job = BackupJob {
            scope: BackupScope::VirtualMachine(Uuid::new_v4()),
            passphrase: Secret::from("very secret passphrase"),
            stop_vms: true,
            include_images: false,
        };
        let options = job.options();
        assert!(options.contains("VIRTUAL_MACHINE"));
        assert!(!options.contains("very secret passphrase"));
    }
}
