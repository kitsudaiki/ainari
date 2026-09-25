pub mod create_ch_virtual_machine;
pub mod delete_ch_virtual_machine;
pub mod reboot_ch_virtual_machine;
pub mod restore_ch_virtual_machine;
pub mod save_ch_virtual_machine;
pub mod start_ch_virtual_machine;
pub mod stop_ch_virtual_machine;

use std::path::Path;

use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::models::VmState;
use cloud_hypervisor_client::{SocketBasedApiClient, socket_based_api_client};
use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use crate::config;
use crate::database::virtual_machine_table;

/// Path of the API-socket of the cloud-hypervisor process of a virtual_machine
///
/// # Arguments
/// * `vm_uuid` - Unique identifier of the virtual_machine
///
/// # Returns
/// * `String` with the path of the API-socket
pub fn vm_socket_path(vm_uuid: &Uuid) -> String {
    format!("{}/cloud-hypervisor.sock", vm_directory(vm_uuid))
}

/// Path of the log-file, where the serial console of a virtual_machine is written to
///
/// # Arguments
/// * `vm_uuid` - Unique identifier of the virtual_machine
///
/// # Returns
/// * `String` with the path of the log-file
pub fn vm_serial_log_path(vm_uuid: &Uuid) -> String {
    format!("{}/serial.log", vm_directory(vm_uuid))
}

/// Directory, which contains the disks and the cloud-init files of a virtual_machine
///
/// # Arguments
/// * `vm_uuid` - Unique identifier of the virtual_machine
///
/// # Returns
/// * `String` with the path of the vm-directory
pub fn vm_directory(vm_uuid: &Uuid) -> String {
    format!(
        "{}/vm_{vm_uuid}",
        config::CONFIG.storage.local_vm_storage_path
    )
}

/// Directory for the temporary files, while the image of a virtual_machine is prepared
///
/// # Arguments
/// * `vm_uuid` - Unique identifier of the virtual_machine
///
/// # Returns
/// * `String` with the path of the temp-directory
pub fn vm_temp_directory(vm_uuid: &Uuid) -> String {
    format!("{}/vm_{vm_uuid}", config::CONFIG.storage.tempfile_location)
}

/// Directory for the temporary files, while a snapshot of a virtual_machine is created or
/// restored
///
/// # Arguments
/// * `uuid` - Unique identifier of the snapshot-operation
///
/// # Returns
/// * `String` with the path of the temp-directory
pub fn snapshot_temp_directory(uuid: &Uuid) -> String {
    format!(
        "{}/snapshot_{uuid}",
        config::CONFIG.storage.tempfile_location
    )
}

/// Connects to the cloud-hypervisor process of an existing virtual_machine and reads its state
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok((SocketBasedApiClient, VmState))` with the client of the API-socket and the current
///   state of the virtual_machine on success
/// * `Err(AinariError)` if the virtual_machine doesn't exist, was only reserved or its
///   cloud-hypervisor process doesn't respond
pub(super) async fn connect_to_vmm(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<(SocketBasedApiClient, VmState), AinariError> {
    virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;

    // a reserved virtual_machine, which was never created, has no cloud-hypervisor process
    let socket_path = vm_socket_path(uuid);
    if !Path::new(&socket_path).exists() {
        return Err(AinariError::InvalidInput(format!(
            "VM {uuid} is not created, so it has no cloud-hypervisor process."
        )));
    }

    let client = socket_based_api_client(&socket_path);
    let vm_info = client
        .vm_info_get()
        .await
        .map_err(|e| AinariError::InternalError(format!("Get info of VM {uuid} failed: {e:?}")))?;

    Ok((client, vm_info.state))
}
