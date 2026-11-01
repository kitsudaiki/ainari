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

mod metadata;
mod plan;
mod prune;
mod restore_dependencies;
mod restore_disk_image;
mod restore_project;
mod restore_virtual_machine;
mod run_restore;

pub use metadata::load_metadata;
pub use plan::{PlanOptions, RestorePlan, calculate_plan, load_target_state};
pub use run_restore::run_restore;

use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

use ainari_common::error::AinariError;

use ainari_api_structs::exports::{HanamiProjectExport, MikoProjectExport, OmamoriProjectExport};

use crate::core::bundle::{RyokanMetadata, SakuraMetadata};
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
    /// Delete the resources of the project, which are not part of the backup
    pub prune: bool,
    /// The decrypted metadata of the backup, which was already checked against the manifest
    pub metadata: BackupMetadata,
    /// Plan of the restore, which was calculated, before the project was locked
    pub plan: RestorePlan,
}

/// Options of a restore-job, like they are stored in the database. The passphrase and the keys
/// are never part of it.
#[derive(Serialize)]
pub struct RestoreJobOptions {
    pub force: bool,
    pub prune: bool,
    pub start_vms: bool,
    pub include_dependencies: bool,
}

impl RestoreJob {
    /// Options of the job, which are stored in the database.
    pub fn options(&self) -> String {
        serde_json::to_string(&RestoreJobOptions {
            force: self.force,
            prune: self.prune,
            start_vms: self.start_vms,
            include_dependencies: self.include_dependencies,
        })
        .unwrap_or_default()
    }
}

/// Decrypted metadata of a backup.
#[derive(Debug, Clone)]
pub struct BackupMetadata {
    /// project and quota, only part of a backup of a project
    pub miko: Option<MikoProjectExport>,
    pub omamori: OmamoriProjectExport,
    /// images, only part of a backup of a project
    pub ryokan: Option<RyokanMetadata>,
    pub hanami: HanamiProjectExport,
    pub sakura: SakuraMetadata,
}

/// State of a running restore.
struct Restorer<'a> {
    job: &'a Job,
    restore: &'a RestoreJob,
    services: &'a Services,
    /// new UUIDs of the resources, whose UUID was already taken
    id_map: BTreeMap<Uuid, Uuid>,
}

impl<'a> Restorer<'a> {
    /// Starts a restore with the UUIDs of its plan, which are stored for the job, so the user can
    /// see them.
    ///
    /// # Returns
    ///
    /// The restorer, or an internal error, if the UUIDs can't be stored.
    fn new(
        job: &'a Job,
        restore: &'a RestoreJob,
        services: &'a Services,
    ) -> Result<Self, AinariError> {
        let mut restorer = Restorer {
            job,
            restore,
            services,
            id_map: BTreeMap::new(),
        };
        for entry in &restore.plan.id_map {
            restorer.map_uuid(&entry.resource_type, entry.old_uuid, entry.new_uuid)?;
        }
        Ok(restorer)
    }

    /// New UUID of a resource of the backup.
    fn new_uuid(&self, old_uuid: &Uuid) -> Uuid {
        *self.id_map.get(old_uuid).unwrap_or(old_uuid)
    }

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
            prune: false,
            start_vms: false,
            include_dependencies: true,
        };
        let json = serde_json::to_string(&options).unwrap();
        assert!(json.contains("\"force\":true"));
        assert!(!json.contains("key"));
    }
}
