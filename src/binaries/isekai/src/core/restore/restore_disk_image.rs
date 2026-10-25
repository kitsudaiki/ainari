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
use uuid::Uuid;

use ainari_api_structs::backup_structs::BackupCopyDirection;
use ainari_api_structs::image_structs::{
    RyokanImageExport, RyokanImageImport, RyokanProjectImportReq,
};
use ainari_api_structs::secret_structs::OmamoriSecretExport;
use ainari_clients::backup as backup_clients;
use ainari_clients::image::import_project_images;
use ainari_common::error::AinariError;

use super::Restorer;
use super::plan::SECRET_OWNER_IMAGE;
use crate::core::bundle::BackupVirtualMachine;

/// Type of the root-disks within a backup
const DISK_FILE_TYPE: &str = "qcow2";

impl Restorer<'_> {
    /// Copies the root-disk of a virtual_machine out of the backup and registers it as
    /// snapshot-image in the project, so sakura can use it like any other snapshot.
    ///
    /// The backup itself stays untouched. The image gets a copy of the secret of the disk, so it
    /// can be deleted again without touching the secret of the backup.
    ///
    /// # Returns
    ///
    /// The UUID of the new snapshot-image.
    pub(super) async fn restore_disk_image(
        &self,
        virtual_machine: &BackupVirtualMachine,
    ) -> Result<Uuid, AinariError> {
        let disk_secret = self
            .restore
            .metadata
            .omamori
            .secrets
            .iter()
            .find(|secret| secret.uuid == virtual_machine.disk.secret_uuid)
            .ok_or_else(|| {
                AinariError::InvalidInput(format!(
                    "Secret '{}' of the disk is not part of the backup.",
                    virtual_machine.disk.secret_uuid
                ))
            })?;

        let image_uuid = Uuid::new_v4();
        let copy = backup_clients::copy_backup_file(
            &self.services.endpoints.ryokan,
            &self.services.token,
            self.services.key(),
            &self.job.backup_uuid,
            BackupCopyDirection::OutOfBackup,
            &virtual_machine.disk.path,
            &image_uuid,
            self.services.insecure(),
        )
        .await?;

        // the payload stays wrapped with the key of the backup and is only bound to the original
        // UUID of the secret, so the copy gets its new UUID by the id-map
        let secret_uuid = Uuid::new_v4();
        let image_secret = OmamoriSecretExport {
            owned_by: SECRET_OWNER_IMAGE.to_string(),
            resource_uuid: Some(image_uuid),
            ..disk_secret.clone()
        };
        self.import_secrets(
            vec![image_secret],
            Vec::new(),
            false,
            BTreeMap::from([(disk_secret.uuid, secret_uuid)]),
        )
        .await?;

        let request = RyokanProjectImportReq {
            images: vec![RyokanImageImport {
                image: RyokanImageExport {
                    uuid: image_uuid,
                    name: snapshot_name(&virtual_machine.config.name),
                    file_type: DISK_FILE_TYPE.to_string(),
                    // the copy is encrypted, so it is a little bit larger than the disk itself
                    file_size: copy.size,
                    is_snapshot: true,
                    secret_uuid,
                },
                onsen_address: copy.onsen_address,
                file_path: copy.file_path,
            }],
            force: false,
            id_map: BTreeMap::new(),
        };
        import_project_images(
            &self.services.endpoints.ryokan,
            &self.services.token,
            self.services.key(),
            &self.job.project_id,
            &request,
            self.services.insecure(),
        )
        .await?;

        Ok(image_uuid)
    }
}

/// Name of the temporary snapshot-image of a restored virtual_machine.
///
/// # Arguments
///
/// * `virtual_machine_name` - Name of the virtual_machine
///
/// # Returns
///
/// The name, which is short enough for an image.
fn snapshot_name(virtual_machine_name: &str) -> String {
    let name: String = virtual_machine_name.chars().take(200).collect();
    format!("restore-{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_name() {
        assert_eq!(snapshot_name("vm"), "restore-vm");
        let long = "ü".repeat(300);
        let name = snapshot_name(&long);
        assert!(name.chars().count() <= 255);
        assert!(name.len() <= 255 * 4);
    }
}
