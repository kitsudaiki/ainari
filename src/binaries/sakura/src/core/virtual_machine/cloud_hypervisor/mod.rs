pub mod create_ch_virtual_machine;
pub mod delete_ch_virtual_machine;

use uuid::Uuid;

use crate::config;

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
