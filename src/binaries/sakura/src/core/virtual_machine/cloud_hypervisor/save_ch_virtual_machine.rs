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

use std::path::Path;

use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::socket_based_api_client;
use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::image::get_image;
use ainari_clients::onsen_file_transfer::upload_file;
use ainari_common::error::AinariError;
use ainari_files::file_encryption::encrypt_file;

use super::create_ch_virtual_machine::{get_secret, run_command};
use super::{snapshot_temp_directory, vm_socket_path};
use crate::config;
use crate::database::virtual_machine_table;

/// Creates a snapshot of the root-disk of a cloud-hypervisor virtual_machine
///
/// The snapshot must already be registered in ryokan as image, which is marked as snapshot.
/// Ryokan also generated the secret for it in omamori. The root-disk is copied into a qcow2-image
/// in the temp-directory, while the virtual_machine is paused, so the disk doesn't change during
/// the copy. Afterwards the image is encrypted with the secret of the snapshot and uploaded into
/// the onsen, which was selected by ryokan.
///
/// The snapshot is only crash-consistent: pausing doesn't flush the page-cache of the guest, so
/// data, which the guest didn't write to the disk yet, is missing in the snapshot (e.g. empty
/// files, which were created shortly before). A clean shutdown or a guest-agent would avoid this,
/// but it was decided to keep the virtual_machine running and to tell the user to run `sync`
/// inside the virtual_machine before the snapshot instead.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `image_uuid` - Unique identifier of the image, which was registered in ryokan as snapshot
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the snapshot was uploaded
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn save_ch_virtual_machine(
    uuid: &Uuid,
    image_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    let virtual_machine_data = virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;
    let root_disk_path = virtual_machine_data.root_disk_path.ok_or_else(|| {
        AinariError::InvalidInput(format!("VM {uuid} has no root-disk, which can be saved."))
    })?;

    let endpoints =
        get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification).await?;

    // get storage-location and secret of the image, which were prepared by ryokan
    let image_resp = get_image(
        &endpoints.ryokan,
        &context.token,
        &config::INTERNAL_API_KEY,
        image_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await?;
    let secret = get_secret(&endpoints, &image_resp.secret_uuid, context).await?;

    log::info!("Start snapshot-image {image_uuid} of VM {uuid}");

    let temp_dir = snapshot_temp_directory(image_uuid);
    create_directory(&temp_dir).await?;
    let local_file_path = format!("{temp_dir}/{image_uuid}");
    let local_encrypted_file_path = format!("{local_file_path}_encrypted");

    let result = async {
        copy_root_disk(uuid, &root_disk_path, &local_file_path).await?;
        encrypt_file(&local_file_path, &local_encrypted_file_path, &secret).await?;
        upload_file(
            &image_resp.onsen_address,
            &image_resp.file_path,
            &local_encrypted_file_path,
        )
        .await
        .map_err(|e| {
            AinariError::InternalError(format!("Failed to upload snapshot-file to onsen: {e}"))
        })
    }
    .await;

    // the temporary files are not required anymore, even if the snapshot failed
    let _ = std::fs::remove_dir_all(&temp_dir).map_err(|e| {
        log::error!("Failed to delete temp-dir {temp_dir} from disk with error {e}.");
    });
    result?;

    log::info!("Snapshot-image {image_uuid} of VM {uuid} uploaded");

    Ok(())
}

/// Copies the root-disk of a virtual_machine into a qcow2-image
///
/// A running virtual_machine is paused during the copy and resumed afterwards, also if the copy
/// failed, so the virtual_machine doesn't stay paused.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `root_disk_path` - Path of the raw root-disk of the virtual_machine
/// * `target_path` - Path of the new qcow2-image
///
/// # Returns
/// * `Ok(())` if the root-disk was copied
/// * `Err(AinariError)` with an appropriate error on failure
async fn copy_root_disk(
    uuid: &Uuid,
    root_disk_path: &str,
    target_path: &str,
) -> Result<(), AinariError> {
    // a virtual_machine without cloud-hypervisor process doesn't write to its disk
    let socket_path = vm_socket_path(uuid);
    let client = if Path::new(&socket_path).exists() {
        let client = socket_based_api_client(&socket_path);
        client
            .pause_vm()
            .await
            .map_err(|e| AinariError::InternalError(format!("Pause VM {uuid} failed: {e:?}")))?;
        Some(client)
    } else {
        None
    };

    // qcow2 skips the unused parts of the raw-disk, so the snapshot is much smaller. It is not
    // compressed, because this would keep the virtual_machine paused for much longer.
    // cloud-hypervisor holds a lock on the whole root-disk as long as its process runs, also while
    // the virtual_machine is paused. qemu-img would fail to lock the disk for reading, even with
    // `-U`, so the locking of qemu-img is disabled for the root-disk. This is safe, because the
    // paused virtual_machine doesn't write to the disk during the copy. Commas are the separator
    // of the image-options, so they are escaped in the path.
    let source_opts = format!(
        "driver=raw,file.driver=file,file.filename={},file.locking=off",
        root_disk_path.replace(',', ",,")
    );
    let convert_result = run_command(
        "qemu-img",
        &[
            "convert",
            "--image-opts",
            &source_opts,
            "-O",
            "qcow2",
            target_path,
        ],
    );

    if let Some(client) = client {
        client
            .resume_vm()
            .await
            .map_err(|e| AinariError::InternalError(format!("Resume VM {uuid} failed: {e:?}")))?;
    }

    convert_result
}
