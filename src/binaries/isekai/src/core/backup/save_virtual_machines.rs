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

use futures::stream::{self, StreamExt};
use std::collections::BTreeMap;

use ainari_api_structs::exports::HanamiVirtualMachineExport;
use ainari_api_structs::secret_structs::SECRET_OWNER_BACKUP;
use ainari_api_structs::task_structs::TaskBackupSaveReq;
use ainari_clients::exports::export_virtual_machine;
use ainari_clients::secret::generate_secret;
use ainari_clients::virtual_machine::{backup_virtual_machine, get_virtual_machine_host};
use ainari_common::error::AinariError;

use super::BackupJob;
use crate::config;
use crate::core::bundle::{BackupDisk, BackupVirtualMachine, SakuraMetadata, disk_path};
use crate::core::processing::jobs::Job;
use crate::core::services::Services;
use crate::database::backup_table;

/// Saves the root-disks of the virtual_machines into the backup.
///
/// The virtual_machines of different sakura-hosts are saved at the same time, but only a limited
/// number of them per host, so a host is not overloaded.
pub(super) async fn save_virtual_machines(
    job: &Job,
    backup: &BackupJob,
    services: &Services,
    virtual_machines: &[HanamiVirtualMachineExport],
) -> Result<SakuraMetadata, AinariError> {
    let mut by_host: BTreeMap<String, Vec<&HanamiVirtualMachineExport>> = BTreeMap::new();
    for virtual_machine in virtual_machines {
        let host = get_virtual_machine_host(
            &services.endpoints.hanami,
            &services.token,
            services.key(),
            &virtual_machine.uuid,
            services.insecure(),
        )
        .await?;
        by_host
            .entry(host.address)
            .or_default()
            .push(virtual_machine);
    }

    let limit = config::CONFIG.processing.max_parallel_snapshots.max(1);
    let per_host = by_host.iter().map(|(address, virtual_machines)| {
        stream::iter(virtual_machines.iter().map(|virtual_machine| {
            save_virtual_machine(job, backup, services, address, virtual_machine)
        }))
        .buffer_unordered(limit)
        .collect::<Vec<_>>()
    });

    let mut saved = Vec::new();
    for result in futures::future::join_all(per_host)
        .await
        .into_iter()
        .flatten()
    {
        saved.push(result?);
    }
    saved.sort_by_key(|virtual_machine| virtual_machine.config.uuid);

    Ok(SakuraMetadata {
        virtual_machines: saved,
    })
}

/// Saves the root-disk of a virtual_machine into the backup.
async fn save_virtual_machine(
    job: &Job,
    backup: &BackupJob,
    services: &Services,
    sakura_address: &str,
    virtual_machine: &HanamiVirtualMachineExport,
) -> Result<BackupVirtualMachine, AinariError> {
    // the config is read before the root-disk is saved, so it has the power-state from before a
    // shutdown for the saving
    let config = export_virtual_machine(
        sakura_address,
        &services.token,
        services.key(),
        &virtual_machine.uuid,
        services.insecure(),
    )
    .await?;

    // every disk gets its own secret, which belongs to the backup and not to the project
    let secret = generate_secret(
        &services.endpoints.omamori,
        &services.token,
        services.key(),
        &format!("backup-{}-disk-{}", job.backup_uuid, virtual_machine.uuid),
        SECRET_OWNER_BACKUP,
        &job.backup_uuid,
        services.insecure(),
    )
    .await?;
    backup_table::add_backup_secret(&job.backup_uuid, &secret.uuid)
        .map_err(|_| AinariError::internal("Failed to store the secret of a disk of the backup"))?;

    let request = TaskBackupSaveReq {
        backup_uuid: job.backup_uuid,
        secret_uuid: secret.uuid,
        stop_vm: backup.stop_vms,
    };
    let task = backup_virtual_machine(
        sakura_address,
        &services.token,
        services.key(),
        &virtual_machine.uuid,
        &request,
        services.insecure(),
    )
    .await?;
    services.wait_for_task(sakura_address, &task).await?;

    Ok(BackupVirtualMachine {
        config,
        disk: BackupDisk {
            path: disk_path(&virtual_machine.uuid),
            secret_uuid: secret.uuid,
        },
    })
}
