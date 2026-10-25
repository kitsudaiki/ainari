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

use std::time::{Duration, Instant};
use uuid::Uuid;

use ainari_api_structs::task_structs::TaskBackupRestoreReq;
use ainari_api_structs::virtual_machine_structs::{
    HanamiVirtualMachineExport, VirtualMachineReserveInternalReq,
};
use ainari_clients::image::delete_image;
use ainari_clients::network::export_project_networks;
use ainari_clients::virtual_machine::{
    create_virtual_machine_task, delete_virtual_machine_in_hanami, get_virtual_machine_host,
    is_virtual_machine_deleted, reserve_virtual_machine_internal,
    restore_virtual_machine_from_backup, stop_virtual_machine_task,
};
use ainari_common::error::AinariError;

use super::Restorer;
use crate::core::bundle::BackupVirtualMachine;

/// Power-state of a running virtual_machine in sakura
const STATE_RUNNING: &str = "RUNNING";
/// Time between two checks, if a deleted virtual_machine is gone on its sakura-host
const DELETE_POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Maximum time, which the deletion of a virtual_machine on its sakura-host may take
const DELETE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

impl<'a> Restorer<'a> {
    /// Restores a virtual_machine of a VM-backup together with its public-key, network,
    /// packet-filters and floating ip-addresses.
    ///
    /// Without `force` an existing virtual_machine with the same UUID stays untouched and the
    /// backup is restored as new virtual_machine under the UUID of the plan. With `force` the
    /// root-disk of the existing virtual_machine is replaced.
    pub(super) async fn restore_virtual_machine(
        &mut self,
        virtual_machine: &BackupVirtualMachine,
    ) -> Result<(), AinariError> {
        let old_uuid = virtual_machine.config.uuid;
        let hanami_vm = self.backup_vm(&old_uuid)?;

        self.job.step(&format!(
            "restore dependencies of virtual_machine {old_uuid}"
        ));
        let target = export_project_networks(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            &self.job.project_id,
            self.services.insecure(),
        )
        .await?;
        let public_key_uuid = self
            .restore_public_key(&virtual_machine.config.public_key_uuid)
            .await?;
        let network_uuid = self
            .restore_network(&target, &hanami_vm.network_uuid)
            .await?;

        self.job
            .step(&format!("restore disk of virtual_machine {old_uuid}"));
        let image_uuid = self.restore_disk_image(virtual_machine).await?;

        let existing = target
            .virtual_machines
            .iter()
            .find(|entry| entry.uuid == old_uuid);
        let result = match existing {
            Some(existing) if self.restore.force => {
                self.job
                    .step(&format!("replace root-disk of virtual_machine {old_uuid}"));
                self.replace_root_disk(existing, hanami_vm, &image_uuid)
                    .await
                    .map(|address| (old_uuid, address))
            }
            _ => {
                self.job.step(&format!("create virtual_machine {old_uuid}"));
                self.create_virtual_machine(
                    hanami_vm,
                    self.new_uuid(&old_uuid),
                    existing.is_none(),
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

        self.job.step(&format!(
            "restore network-filters of virtual_machine {old_uuid}"
        ));
        self.restore_network_filters(&old_uuid).await?;

        self.apply_power_state(virtual_machine, &new_uuid, &sakura_address)
            .await
    }

    /// Virtual_machine within the metadata of hanami of the backup.
    pub(super) fn backup_vm(
        &self,
        virtual_machine_uuid: &Uuid,
    ) -> Result<&'a HanamiVirtualMachineExport, AinariError> {
        self.restore
            .metadata
            .hanami
            .virtual_machines
            .iter()
            .find(|entry| entry.uuid == *virtual_machine_uuid)
            .ok_or_else(|| {
                AinariError::InvalidInput(format!(
                    "Virtual_machine '{virtual_machine_uuid}' is missing in the metadata of hanami."
                ))
            })
    }

    /// Deletes the snapshot-image, which was only required to create the root-disk of a
    /// virtual_machine. A failure is only logged, because the virtual_machine is already restored.
    pub(super) async fn delete_temporary_image(&self, image_uuid: &Uuid) {
        if let Err(e) = delete_image(
            &self.services.endpoints.ryokan,
            &self.services.token,
            self.services.key(),
            image_uuid,
            self.services.insecure(),
        )
        .await
        {
            log::warn!("Failed to delete the temporary snapshot-image {image_uuid}: {e}");
        }
    }

    /// Stops a restored virtual_machine, if it was not running at the time of the backup, unless
    /// all virtual_machines are started by the restore.
    ///
    /// # Arguments
    ///
    /// * `virtual_machine` - The virtual_machine within the backup
    /// * `uuid` - UUID of the restored virtual_machine
    /// * `sakura_address` - Internal address of the sakura-host of the restored virtual_machine
    pub(super) async fn apply_power_state(
        &self,
        virtual_machine: &BackupVirtualMachine,
        uuid: &Uuid,
        sakura_address: &str,
    ) -> Result<(), AinariError> {
        if virtual_machine.config.vm_state == STATE_RUNNING || self.restore.start_vms {
            return Ok(());
        }
        self.job.step(&format!("stop virtual_machine {uuid}"));
        let task = stop_virtual_machine_task(
            sakura_address,
            &self.services.token,
            self.services.key(),
            uuid,
            self.services.insecure(),
        )
        .await?;
        self.services.wait_for_task(sakura_address, &task).await
    }

    /// Replaces the root-disk of an existing virtual_machine with the one of the backup.
    ///
    /// # Returns
    ///
    /// The internal address of the sakura-host of the virtual_machine.
    pub(super) async fn replace_root_disk(
        &self,
        existing: &HanamiVirtualMachineExport,
        backup: &HanamiVirtualMachineExport,
        image_uuid: &Uuid,
    ) -> Result<String, AinariError> {
        // the root-disk is replaced as a whole, so it must have the same size
        if existing.disk_size != backup.disk_size {
            return Err(AinariError::Conflict(format!(
                "Virtual_machine '{}' has a disk of {} GiB, but the backup a disk of {} GiB. \
                 Delete the virtual_machine, before the backup is restored.",
                existing.uuid, existing.disk_size, backup.disk_size
            )));
        }

        let host = get_virtual_machine_host(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            &existing.uuid,
            self.services.insecure(),
        )
        .await?;
        let request = TaskBackupRestoreReq {
            backup_uuid: self.job.backup_uuid,
            image_uuid: *image_uuid,
        };
        let task = restore_virtual_machine_from_backup(
            &host.address,
            &self.services.token,
            self.services.key(),
            &existing.uuid,
            &request,
            self.services.insecure(),
        )
        .await?;
        self.services.wait_for_task(&host.address, &task).await?;

        Ok(host.address)
    }

    /// Creates the virtual_machine of the backup as new virtual_machine from the snapshot-image.
    ///
    /// It gets the given UUID and, if requested, its original addresses. If the addresses are
    /// used by another virtual_machine, hanami selects new ones, and if also the UUID is used, a
    /// new UUID.
    ///
    /// # Returns
    ///
    /// The UUID of the new virtual_machine and the internal address of its sakura-host.
    pub(super) async fn create_virtual_machine(
        &self,
        backup: &HanamiVirtualMachineExport,
        uuid: Uuid,
        keep_addresses: bool,
        network_uuid: &Uuid,
        image_uuid: &Uuid,
        public_key_uuid: &Uuid,
    ) -> Result<(Uuid, String), AinariError> {
        let number_of_cores = i32::try_from(backup.number_of_cores).map_err(|_| {
            AinariError::InvalidInput(format!(
                "Virtual_machine '{}' has an invalid number of cores.",
                backup.uuid
            ))
        })?;
        let mut request = VirtualMachineReserveInternalReq {
            uuid: Some(uuid),
            name: backup.name.clone(),
            network_uuid: *network_uuid,
            number_of_cores,
            memory_size: backup.memory_size,
            disk_size: backup.disk_size,
            internal_ip: keep_addresses.then_some(backup.internal_ip),
            mac_address: keep_addresses.then(|| backup.mac_address.clone()),
            isolated_host: false,
        };

        let mut result = self.reserve(&request).await;
        // the addresses or the UUID are used by another virtual_machine
        if matches!(result, Err(AinariError::Conflict(_))) && keep_addresses {
            log::info!(
                "Restore virtual_machine '{}' with new addresses",
                backup.uuid
            );
            request.internal_ip = None;
            request.mac_address = None;
            result = self.reserve(&request).await;
        }
        if matches!(result, Err(AinariError::Conflict(_))) {
            log::info!("Restore virtual_machine '{}' with new UUID", backup.uuid);
            request.uuid = None;
            result = self.reserve(&request).await;
        }
        let reserved = result?;

        let host = get_virtual_machine_host(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            &reserved,
            self.services.insecure(),
        )
        .await?;
        let task = create_virtual_machine_task(
            &host.address,
            &self.services.token,
            self.services.key(),
            &reserved,
            image_uuid,
            public_key_uuid,
            self.services.insecure(),
        )
        .await?;
        self.services.wait_for_task(&host.address, &task).await?;

        Ok((reserved, host.address))
    }

    /// Deletes a virtual_machine of the project and waits, until it is gone on its sakura-host,
    /// so its UUID and resources can be used again.
    ///
    /// A virtual_machine, which doesn't exist anymore, is skipped, so the deletion can be
    /// repeated.
    pub(super) async fn delete_virtual_machine(&self, uuid: &Uuid) -> Result<(), AinariError> {
        let host = match get_virtual_machine_host(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            uuid,
            self.services.insecure(),
        )
        .await
        {
            Ok(host) => host,
            Err(AinariError::NotFound(_)) => return Ok(()),
            Err(e) => return Err(e),
        };
        delete_virtual_machine_in_hanami(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            uuid,
            self.services.insecure(),
        )
        .await?;

        let start = Instant::now();
        while !is_virtual_machine_deleted(
            &host.address,
            &self.services.token,
            self.services.key(),
            uuid,
            self.services.insecure(),
        )
        .await?
        {
            if start.elapsed() > DELETE_TIMEOUT {
                return Err(AinariError::internal(format!(
                    "Virtual_machine '{uuid}' was not deleted in time"
                )));
            }
            tokio::time::sleep(DELETE_POLL_INTERVAL).await;
        }
        Ok(())
    }

    /// Reserves a virtual_machine in hanami.
    async fn reserve(
        &self,
        request: &VirtualMachineReserveInternalReq,
    ) -> Result<Uuid, AinariError> {
        reserve_virtual_machine_internal(
            &self.services.endpoints.hanami,
            &self.services.token,
            self.services.key(),
            request,
            self.services.insecure(),
        )
        .await
        .map(|virtual_machine| virtual_machine.uuid)
    }
}
