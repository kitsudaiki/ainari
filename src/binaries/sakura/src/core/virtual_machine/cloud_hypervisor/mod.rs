pub mod create_ch_virtual_machine;
pub mod delete_ch_virtual_machine;
pub mod reboot_ch_virtual_machine;
pub mod restart_after_host_restart;
pub mod restore_ch_virtual_machine;
pub mod save_ch_virtual_machine;
pub mod start_ch_virtual_machine;
pub mod stop_ch_virtual_machine;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use uuid::Uuid;

use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::models::VmState;
use cloud_hypervisor_client::{SocketBasedApiClient, socket_based_api_client};

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use crate::config;
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::VirtualMachineState;

/// Time, which the guest gets to shut itself down after the power-button was pressed, before it
/// is powered off hard
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(60);

/// Interval, in which the state of the virtual_machine is checked while it shuts down
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Runs an external command and waits until it is finished
///
/// # Arguments
/// * `program` - Name or path of the program to run
/// * `args` - Arguments for the program
///
/// # Returns
/// * `Ok(())` if the command was successful
/// * `Err(AinariError)` if the command could not be started or failed
pub fn run_command(program: &str, args: &[&str]) -> Result<(), AinariError> {
    let status = Command::new(program).args(args).status().map_err(|e| {
        AinariError::InternalError(format!("Failed to execute {program} process: {e}"))
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(AinariError::InternalError(format!(
            "{program} failed with exit status: {status}"
        )))
    }
}

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
pub async fn connect_to_vmm(
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

/// Stores a new state of a virtual_machine in the database
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `state` - New state of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the state was stored
/// * `Err(AinariError)` if the virtual_machine doesn't exist or the database failed
pub fn set_vm_state(
    uuid: &Uuid,
    state: VirtualMachineState,
    context: &UserContext,
) -> Result<(), AinariError> {
    log::debug!("Set state of VM {uuid} to {state}");
    virtual_machine_table::update_virtual_machine_state(uuid, &state, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))
}

/// Marks a virtual_machine as error, if an operation failed, which should bring it into the
/// running state
///
/// Rejected requests, which are signaled by `AinariError::InvalidInput`, don't change the state,
/// because the virtual_machine was not touched by them. The original result is always returned,
/// also if the error-state could not be stored.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `context` - User context containing authentication information
/// * `result` - Result of the operation
///
/// # Returns
/// * The unchanged `result` of the operation
pub fn mark_error_on_failure<T>(
    uuid: &Uuid,
    context: &UserContext,
    result: Result<T, AinariError>,
) -> Result<T, AinariError> {
    if let Err(e) = &result {
        if !matches!(e, AinariError::InvalidInput(_)) {
            if let Err(state_err) = set_vm_state(uuid, VirtualMachineState::Error, context) {
                log::error!("Failed to mark VM {uuid} as error: {state_err}");
            }
        }
    }
    result
}

/// Shuts a booted virtual_machine down like a normal shutdown of a real machine
///
/// The power-button is pressed, so the guest shuts itself down and writes all its data to the
/// disk. Only if the guest doesn't power itself off within `GRACEFUL_SHUTDOWN_TIMEOUT`, it is
/// powered off hard, which can lose data, that the guest didn't write to its disk yet.
///
/// cloud-hypervisor exits, when the guest powers itself off, so the virtual_machine has to be
/// created in a new process to be started again. After the hard power-off the process keeps
/// running with the shut down virtual_machine instead.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `client` - Client of the API-socket of the cloud-hypervisor process
/// * `state` - Current state of the virtual_machine
///
/// # Returns
/// * `Ok(())` if the virtual_machine is shut down
/// * `Err(AinariError)` if it could not even be powered off hard
pub(super) async fn shutdown_gracefully(
    uuid: &Uuid,
    client: &SocketBasedApiClient,
    state: VmState,
) -> Result<(), AinariError> {
    // a paused guest can not react on the power-button
    if state == VmState::Paused {
        if let Err(e) = client.resume_vm().await {
            log::warn!("Resume of paused VM {uuid} before its shutdown failed: {e:?}");
        }
    }

    log::info!("Press power-button of VM {uuid}");
    match client.power_button_vm().await {
        Ok(()) => {
            if wait_for_shutdown(uuid, client, GRACEFUL_SHUTDOWN_TIMEOUT).await {
                log::info!("VM {uuid} shut itself down");
                return Ok(());
            }
            log::warn!(
                "VM {uuid} didn't shut itself down within {}s after the power-button, so it is \
                 powered off hard",
                GRACEFUL_SHUTDOWN_TIMEOUT.as_secs()
            );
        }
        Err(e) => {
            log::warn!("Power-button of VM {uuid} failed: {e:?}, so it is powered off hard");
        }
    }

    match client.shutdown_vm().await {
        Ok(()) => Ok(()),
        // the guest could have powered itself off in the meantime
        Err(_) if vmm_exited(uuid) => Ok(()),
        Err(e) => Err(AinariError::InternalError(format!(
            "Shutdown VM {uuid} failed: {e:?}"
        ))),
    }
}

/// Waits until the virtual_machine is shut down
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `client` - Client of the API-socket of the cloud-hypervisor process
/// * `timeout` - Maximum time to wait
///
/// # Returns
/// True, if the virtual_machine is shut down or its cloud-hypervisor process exited in time
async fn wait_for_shutdown(uuid: &Uuid, client: &SocketBasedApiClient, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if vmm_exited(uuid) {
            return true;
        }
        if let Ok(vm_info) = client.vm_info_get().await {
            if vm_info.state == VmState::Shutdown {
                return true;
            }
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(SHUTDOWN_POLL_INTERVAL).await;
    }
}

/// Checks if the cloud-hypervisor process of a virtual_machine exited
///
/// The process removes its API-socket, when it exits.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
///
/// # Returns
/// True, if the process exited
pub(super) fn vmm_exited(uuid: &Uuid) -> bool {
    !Path::new(&vm_socket_path(uuid)).exists()
}
