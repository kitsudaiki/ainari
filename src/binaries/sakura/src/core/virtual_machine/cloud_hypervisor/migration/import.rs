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

use cloud_hypervisor_client::apis::DefaultApi;
use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::migration_structs::MigrationFile;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use super::super::create_ch_virtual_machine::spawn_vmm_with_vm;
use super::super::{mark_error_on_failure, set_vm_state, vm_directory};
use super::transfer::receive_file;
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::VirtualMachineState;

/// Takes over a virtual_machine, which was exported by the source host
///
/// The database-entry of the virtual_machine was already created with the same identity as on
/// the source by the import-endpoint. Its seed-image and its root-disk are pulled from the source
/// and it is booted afterwards, if it was running before the migration. The seed-image is copied
/// instead of being built again, so the public-key of the virtual_machine is not required here.
///
/// The virtual_machine is marked as error, if the import failed, so hanami removes it again.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `source_address` - Internal address of the source host
/// * `boot` - Boot the virtual_machine after the import
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine was taken over
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn import_ch_virtual_machine(
    uuid: &Uuid,
    source_address: &str,
    boot: bool,
    context: &UserContext,
) -> Result<(), AinariError> {
    let result = import(uuid, source_address, boot, context).await;
    mark_error_on_failure(uuid, context, result)
}

/// Pulls the files of the virtual_machine and starts it, if requested
async fn import(
    uuid: &Uuid,
    source_address: &str,
    boot: bool,
    context: &UserContext,
) -> Result<(), AinariError> {
    let vm_dir = vm_directory(uuid);
    create_directory(&vm_dir).await?;

    // the same paths, which a newly created virtual_machine gets
    let seed_path = format!("{vm_dir}/seed.iso");
    let root_disk_path = format!("{vm_dir}/boot_disk");

    log::info!("Import VM {uuid} from {source_address}");
    receive_file(
        source_address,
        &context.token,
        uuid,
        MigrationFile::Seed,
        &seed_path,
    )
    .await?;
    receive_file(
        source_address,
        &context.token,
        uuid,
        MigrationFile::RootDisk,
        &root_disk_path,
    )
    .await?;
    log::info!("Received the files of VM {uuid}");

    let virtual_machine_data = virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;
    virtual_machine_table::update_virtual_machine(
        uuid,
        &virtual_machine_data.image_uuid,
        &virtual_machine_data.public_key_uuid,
        &seed_path,
        Some(root_disk_path.clone()),
        context,
    )
    .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;

    if !boot {
        set_vm_state(uuid, VirtualMachineState::Stopped, context)?;
        log::info!("VM {uuid} imported, it stays stopped like on the source");
        return Ok(());
    }

    let (client, _) =
        spawn_vmm_with_vm(uuid, &virtual_machine_data, &root_disk_path, &seed_path).await?;
    client
        .boot_vm()
        .await
        .map_err(|e| AinariError::InternalError(format!("Boot VM {uuid} failed: {e:?}")))?;
    set_vm_state(uuid, VirtualMachineState::Running, context)?;

    log::info!("VM {uuid} imported and started");
    Ok(())
}
