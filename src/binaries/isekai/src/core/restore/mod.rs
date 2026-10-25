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

//! Job-steps of a restore.
//!
//! The passphrase was already checked against the manifest by the endpoint and the project was
//! locked, before the job was queued. The job checks the files of the backup first, so a
//! manipulated backup is rejected before anything is changed. A restore, which fails after it
//! has changed the project, keeps the project locked, so nobody works on a half restored project.

mod restore_dependencies;
mod restore_disk_image;
mod restore_virtual_machine;
mod run_restore;

pub use run_restore::run_restore;

use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

use ainari_common::error::AinariError;

use ainari_api_structs::network_structs::HanamiProjectExport;
use ainari_api_structs::project_structs::OmamoriProjectExport;

use crate::core::bundle::SakuraMetadata;
use crate::core::keys::BackupKeys;
use crate::core::manifest::Manifest;
use crate::core::processing::jobs::Job;
use crate::core::services::Services;
use crate::database::id_map_table::{self, IdMapEntry};

/// Restore, which is processed by a job.
#[derive(Debug)]
pub struct RestoreJob {
    /// Keys of the backup, which were derived from the passphrase. They only exist in the memory
    /// of the job.
    pub keys: BackupKeys,
    /// The authenticated manifest of the backup
    pub manifest: Manifest,
    /// Overwrite existing resources with the same UUID
    pub force: bool,
    /// Start all restored virtual_machines, independent of their power-state in the backup
    pub start_vms: bool,
    /// Create the network of a virtual_machine from the backup, if it doesn't exist anymore
    pub include_dependencies: bool,
}

/// Options of a restore-job, like they are stored in the database. The passphrase and the keys
/// are never part of it.
#[derive(Serialize)]
pub struct RestoreJobOptions {
    pub force: bool,
    pub start_vms: bool,
    pub include_dependencies: bool,
}

impl RestoreJob {
    /// Options of the job, which are stored in the database.
    pub fn options(&self) -> String {
        serde_json::to_string(&RestoreJobOptions {
            force: self.force,
            start_vms: self.start_vms,
            include_dependencies: self.include_dependencies,
        })
        .unwrap_or_default()
    }
}

/// Decrypted metadata of a backup.
pub struct BackupMetadata {
    pub omamori: OmamoriProjectExport,
    pub hanami: HanamiProjectExport,
    pub sakura: SakuraMetadata,
}

/// State of a running restore.
struct Restorer<'a> {
    job: &'a Job,
    restore: &'a RestoreJob,
    services: &'a Services,
    metadata: &'a BackupMetadata,
    /// new UUIDs of the resources, whose UUID was already taken
    id_map: BTreeMap<Uuid, Uuid>,
}

impl Restorer<'_> {
    /// Stores the new UUID of a resource, if it differs from its UUID in the backup.
    pub(super) fn map_uuid(
        &mut self,
        resource_type: &str,
        old_uuid: Uuid,
        new_uuid: Uuid,
    ) -> Result<(), AinariError> {
        if old_uuid == new_uuid {
            return Ok(());
        }
        self.id_map.insert(old_uuid, new_uuid);
        id_map_table::put_id_map_entry(IdMapEntry {
            job_uuid: self.job.uuid,
            resource_type: resource_type.to_string(),
            old_uuid,
            new_uuid,
        })
        .map_err(|e| {
            AinariError::internal(format!("Failed to store the id-map of the job: {e}"))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_contain_no_keys() {
        let options = RestoreJobOptions {
            force: true,
            start_vms: false,
            include_dependencies: true,
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"force\":true"));
        assert!(!json.contains("key"));
    }
}
