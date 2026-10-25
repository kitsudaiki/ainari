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

use uuid::Uuid;

use ainari_api_structs::backup_structs::BackupCopyDirection;
use ainari_api_structs::image_structs::{RyokanImageImport, RyokanProjectImportReq};
use ainari_api_structs::network_structs::HanamiProjectImportReq;
use ainari_clients::backup as backup_clients;
use ainari_clients::image::import_project_images;
use ainari_clients::network::export_project_networks;
use ainari_clients::project::import_project;
use ainari_common::error::AinariError;

use super::Restorer;
use super::plan::{images_restored, restored_secrets};
use crate::core::bundle::{BackupVirtualMachine, image_path};

impl Restorer<'_> {
    /// Restores all resources of a project in the order of their dependencies.
    ///
    /// The project itself was already created by the endpoint, if it didn't exist. With `prune`
    /// the resources, which are not part of the backup, are deleted first. Every step can be
    /// repeated by another restore with `force`, which overwrites the resources, which were
    /// already restored.
    pub(super) async fn restore_project(&mut self) -> Result<(), AinariError> {
        let force = self.restore.force;
        let metadata = &self.restore.metadata;

        // a project, which was created by the endpoint, already has the name and quota of the
        // backup. An existing one is only changed with `force`.
        if force
            && !self.restore.plan.create_project
            && let Some(project) = &metadata.miko
        {
            self.job.step("restore project");
            import_project(
                &self.services.miko,
                &self.services.token,
                self.services.key(),
                &self.job.project_id,
                project,
                true,
                self.services.insecure(),
            )
            .await?;
        }

        if self.restore.prune {
            self.prune().await?;
        }

        self.job.step("restore secrets and public-keys");
        self.import_secrets(
            restored_secrets(metadata),
            metadata.omamori.public_keys.clone(),
            force,
            self.id_map.clone(),
        )
        .await?;

        self.restore_images().await?;

        self.job.step("restore networks");
        if !metadata.hanami.networks.is_empty() {
            self.import_networks(&HanamiProjectImportReq {
                networks: metadata.hanami.networks.clone(),
                network_filters: Vec::new(),
                floating_ips: Vec::new(),
                force,
                id_map: self.id_map.clone(),
            })
            .await?;
        }

        let mut restored = Vec::new();
        for virtual_machine in &metadata.sakura.virtual_machines {
            let (uuid, sakura_address) = self
                .restore_project_virtual_machine(virtual_machine)
                .await?;
            restored.push((virtual_machine, uuid, sakura_address));
        }

        // the packet-filters and floating ip-addresses need the restored virtual_machines
        self.job
            .step("restore network-filters and floating ip-addresses");
        if !metadata.hanami.network_filters.is_empty() || !metadata.hanami.floating_ips.is_empty() {
            self.import_networks(&HanamiProjectImportReq {
                networks: Vec::new(),
                network_filters: metadata.hanami.network_filters.clone(),
                floating_ips: metadata.hanami.floating_ips.clone(),
                force,
                id_map: self.id_map.clone(),
            })
            .await?;
        }

        for (virtual_machine, uuid, sakura_address) in restored {
            self.apply_power_state(virtual_machine, &uuid, &sakura_address)
                .await?;
        }

        Ok(())
    }

    /// Copies the files of the images out of the backup to normal onsens and registers them in
    /// ryokan.
    ///
    /// Every copy gets a new UUID for its file, so it never collides with the file of an existing
    /// image. The images themselves get the UUIDs of the plan.
    async fn restore_images(&self) -> Result<(), AinariError> {
        let Some(ryokan) = &self.restore.metadata.ryokan else {
            return Ok(());
        };
        if !images_restored(&self.restore.metadata) || ryokan.images.is_empty() {
            return Ok(());
        }

        self.job.step("restore images");
        let mut images = Vec::new();
        for image in &ryokan.images {
            let copy = backup_clients::copy_backup_file(
                &self.services.endpoints.ryokan,
                &self.services.token,
                self.services.key(),
                &self.job.backup_uuid,
                BackupCopyDirection::OutOfBackup,
                &image_path(&image.uuid),
                &Uuid::new_v4(),
                self.services.insecure(),
            )
            .await?;
            images.push(RyokanImageImport {
                image: image.clone(),
                onsen_address: copy.onsen_address,
                file_path: copy.file_path,
            });
        }

        let request = RyokanProjectImportReq {
            images,
            force: self.restore.force,
            id_map: self.id_map.clone(),
        };
        import_project_images(
            &self.services.endpoints.ryokan,
            &self.services.token,
            self.services.key(),
            &self.job.project_id,
            &request,
            self.services.insecure(),
        )
        .await
        .map(|_| ())
    }

    /// Restores a virtual_machine of a project-backup under the UUID of the plan.
    ///
    /// Its public-key and network were already restored. An existing virtual_machine with the
    /// same UUID, which only exists with `force`, gets the root-disk of the backup. If its
    /// disk-size changed, it is deleted and created again.
    ///
    /// # Returns
    ///
    /// The UUID of the restored virtual_machine and the internal address of its sakura-host.
    async fn restore_project_virtual_machine(
        &mut self,
        virtual_machine: &BackupVirtualMachine,
    ) -> Result<(Uuid, String), AinariError> {
        let old_uuid = virtual_machine.config.uuid;
        let hanami_vm = self.backup_vm(&old_uuid)?;
        let uuid = self.new_uuid(&old_uuid);
        let network_uuid = self.new_uuid(&hanami_vm.network_uuid);
        let public_key_uuid = self.new_uuid(&virtual_machine.config.public_key_uuid);

        let target = export_project_networks(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            &self.job.project_id,
            self.services.insecure(),
        )
        .await?;
        let existing = target
            .virtual_machines
            .into_iter()
            .find(|entry| entry.uuid == uuid);

        self.job
            .step(&format!("restore disk of virtual_machine {old_uuid}"));
        let image_uuid = self.restore_disk_image(virtual_machine).await?;

        let result = match existing {
            Some(existing) if existing.disk_size == hanami_vm.disk_size => {
                self.job
                    .step(&format!("replace root-disk of virtual_machine {uuid}"));
                self.replace_root_disk(&existing, hanami_vm, &image_uuid)
                    .await
                    .map(|address| (uuid, address))
            }
            existing => {
                if existing.is_some() {
                    self.job.step(&format!(
                        "delete virtual_machine {uuid} with other disk-size"
                    ));
                    self.delete_virtual_machine(&uuid).await?;
                }
                self.job.step(&format!("create virtual_machine {uuid}"));
                self.create_virtual_machine(
                    hanami_vm,
                    uuid,
                    true,
                    &network_uuid,
                    &image_uuid,
                    &public_key_uuid,
                )
                .await
            }
        };
        self.delete_temporary_image(&image_uuid).await;
        let (new_uuid, sakura_address) = result?;
        self.map_uuid("virtual_machine", old_uuid, new_uuid)?;

        Ok((new_uuid, sakura_address))
    }
}
