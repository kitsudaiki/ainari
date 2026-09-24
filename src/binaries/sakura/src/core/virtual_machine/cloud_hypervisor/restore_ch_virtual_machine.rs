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

use std::fs;
use std::path::Path;

use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::socket_based_api_client;
use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::snapshot_structs::SnapshotInternalResp;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::onsen_file_transfer::download_file;
use ainari_clients::snapshot::get_snapshot;
use ainari_common::error::AinariError;
use ainari_common::secret::Secret;
use ainari_files::file_encryption::decrypt_file;

use super::create_ch_virtual_machine::{get_secret, run_command};
use super::{snapshot_temp_directory, vm_socket_path};
use crate::config;
use crate::database::virtual_machine_table;

/// Resets the root-disk of an existing cloud-hypervisor virtual_machine to the state of a snapshot
///
/// A running virtual_machine is shut down first. Then the snapshot is downloaded from the onsen
/// into the temp-directory, decrypted with its secret from omamori and converted into a new
/// raw-disk, which replaces the root-disk. At the end the virtual_machine is booted again, also
/// if the restore failed, so it doesn't stay down. In case of a failure the old root-disk is
/// kept, because it is only replaced after the new disk was completely written.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to reset
/// * `snapshot_uuid` - Unique identifier of the snapshot to restore
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the root-disk was replaced by the snapshot
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn restore_ch_virtual_machine(
    uuid: &Uuid,
    snapshot_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    let virtual_machine_data = virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;
    let root_disk_path = virtual_machine_data.root_disk_path.ok_or_else(|| {
        AinariError::InvalidInput(format!(
            "VM {uuid} has no root-disk, which can be restored."
        ))
    })?;

    let endpoints =
        get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification).await?;

    // get storage-location and secret of the snapshot, before the virtual_machine is stopped, so
    // an invalid snapshot doesn't stop it at all
    let snapshot_resp = get_snapshot(
        &endpoints.ryokan,
        &context.token,
        &config::INTERNAL_API_KEY,
        snapshot_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await?;
    let secret = get_secret(&endpoints, &snapshot_resp.secret_uuid, context).await?;

    log::info!("Start restore of snapshot {snapshot_uuid} into VM {uuid}");

    // a virtual_machine without cloud-hypervisor process doesn't use its disk
    let socket_path = vm_socket_path(uuid);
    let client = if Path::new(&socket_path).exists() {
        let client = socket_based_api_client(&socket_path);
        client
            .shutdown_vm()
            .await
            .map_err(|e| AinariError::InternalError(format!("Shutdown VM {uuid} failed: {e:?}")))?;
        Some(client)
    } else {
        None
    };

    let restore_result =
        replace_root_disk(snapshot_uuid, &snapshot_resp, &secret, &root_disk_path).await;

    // the cloud-hypervisor process keeps the configuration of the virtual_machine while it is
    // shut down and opens the disk again at boot, so the virtual_machine starts with the new disk
    if let Some(client) = client {
        client
            .boot_vm()
            .await
            .map_err(|e| AinariError::InternalError(format!("Boot VM {uuid} failed: {e:?}")))?;
    }
    restore_result?;

    log::info!("Snapshot {snapshot_uuid} restored into VM {uuid}");

    Ok(())
}

/// Replaces the root-disk of a stopped virtual_machine by the content of a snapshot
///
/// The snapshot is downloaded, decrypted and converted into a new raw-disk next to the
/// root-disk, which replaces the root-disk by a rename at the end.
///
/// # Arguments
/// * `snapshot_uuid` - Unique identifier of the snapshot to restore
/// * `snapshot_resp` - Storage-location of the snapshot from ryokan
/// * `secret` - Secret to decrypt the snapshot
/// * `root_disk_path` - Path of the root-disk, which is replaced
///
/// # Returns
/// * `Ok(())` if the root-disk was replaced
/// * `Err(AinariError)` with an appropriate error on failure
async fn replace_root_disk(
    snapshot_uuid: &Uuid,
    snapshot_resp: &SnapshotInternalResp,
    secret: &Secret,
    root_disk_path: &str,
) -> Result<(), AinariError> {
    // the same snapshot can be restored into multiple virtual_machines at the same time, so each
    // restore gets its own temp-directory
    let temp_dir = snapshot_temp_directory(&Uuid::new_v4());
    create_directory(&temp_dir).await?;
    let local_file_path = format!("{temp_dir}/{snapshot_uuid}");
    let local_encrypted_file_path = format!("{local_file_path}_encrypted");

    // the new disk is placed next to the root-disk, so it can replace the root-disk by a rename
    let new_root_disk_path = format!("{root_disk_path}.restore");

    let result = async {
        download_file(
            &snapshot_resp.onsen_address,
            &snapshot_resp.file_path,
            &local_encrypted_file_path,
        )
        .await
        .map_err(|e| {
            AinariError::InternalError(format!("Failed to download snapshot-file from onsen: {e}"))
        })?;
        decrypt_file(&local_encrypted_file_path, &local_file_path, secret).await?;
        run_command(
            "qemu-img",
            &[
                "convert",
                "-f",
                "qcow2",
                "-O",
                "raw",
                &local_file_path,
                &new_root_disk_path,
            ],
        )?;
        rename_disk(&new_root_disk_path, root_disk_path)
    }
    .await;

    // the temporary files are not required anymore, even if the restore failed
    let _ = std::fs::remove_dir_all(&temp_dir).map_err(|e| {
        log::error!("Failed to delete temp-dir {temp_dir} from disk with error {e}.");
    });
    if result.is_err() {
        let _ = fs::remove_file(&new_root_disk_path);
    }

    result
}

/// Moves a disk to a new path and overwrites the file, which is already there
///
/// # Arguments
/// * `source_path` - Current path of the disk
/// * `target_path` - New path of the disk
///
/// # Returns
/// * `Ok(())` if the disk was moved
/// * `Err(AinariError)` if the disk could not be moved
fn rename_disk(source_path: &str, target_path: &str) -> Result<(), AinariError> {
    fs::rename(source_path, target_path).map_err(|e| {
        AinariError::InternalError(format!(
            "Failed to move disk {source_path} to {target_path}: {e}"
        ))
    })
}
