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

use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::socket_based_api_client;
use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::DbError;
use ainari_common::error::AinariError;

use super::{vm_directory, vm_serial_log_path, vm_socket_path, vm_temp_directory};
use crate::database::virtual_machine_table;

/// Deletes a cloud-hypervisor virtual_machine completely from this host
///
/// The cloud-hypervisor process of the virtual_machine is stopped, all of its files, like the
/// boot-disk, the cloud-init seed-image, the API-socket and the serial-log, are removed and at
/// the end it is marked as deleted in the database. The database-entry is changed last, because
/// hanami watches it to know, when the virtual_machine is really gone.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to delete
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine was deleted or was already deleted before
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn delete_ch_virtual_machine(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    // a virtual_machine, which is not in the database anymore, was already deleted by an earlier
    // task, so there is nothing left to do
    match virtual_machine_table::get_virtual_machine(uuid, context) {
        Ok(_) => {}
        Err(DbError::NotFound) => {
            log::warn!("VM {uuid} is already deleted, so there is nothing to delete.");
            return Ok(());
        }
        Err(e) => {
            return Err(map_db_uuid_get_delete_ainari_error(
                "virtual_machine",
                uuid,
                e,
            ));
        }
    }

    log::info!("Start deletion of VM {uuid}");

    // a reserved virtual_machine, which was never created, has no running process
    let socket_path = vm_socket_path(uuid);
    if Path::new(&socket_path).exists() {
        stop_vmm(uuid, &socket_path).await?;
    }

    remove_file(&socket_path)?;
    remove_file(&vm_serial_log_path(uuid))?;
    remove_directory(&vm_directory(uuid))?;
    remove_directory(&vm_temp_directory(uuid))?;

    virtual_machine_table::delete_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;

    log::info!("VM {uuid} deleted");

    Ok(())
}

/// Stops a virtual_machine and the cloud-hypervisor process, which runs it
///
/// The virtual_machine is shut down via the API-socket first. If the cloud-hypervisor process
/// doesn't react on its API, it is killed instead.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `socket_path` - Path of the API-socket of the cloud-hypervisor process
///
/// # Returns
/// * `Ok(())` if the cloud-hypervisor process is stopped
/// * `Err(AinariError)` if the process could not be killed
async fn stop_vmm(uuid: &Uuid, socket_path: &str) -> Result<(), AinariError> {
    let client = socket_based_api_client(socket_path);

    // a failed shutdown of the virtual_machine is not critical, because the shutdown of the
    // cloud-hypervisor process stops the virtual_machine anyway
    if let Err(e) = client.shutdown_vm().await {
        log::warn!("Shutdown of VM {uuid} failed: {e:?}");
    }

    // the cloud-hypervisor process exits after this call
    match client.shutdown_vmm().await {
        Ok(_) => {
            log::info!("Gracefully shut down VMM of VM {uuid}");
            Ok(())
        }
        Err(e) => {
            log::warn!("API shutdown of VMM of VM {uuid} failed: {e:?}. Forcing kill...");
            kill_vmm(socket_path)
        }
    }
}

/// Kills the cloud-hypervisor process, which listens on the given API-socket
///
/// The process is found by its command-line, because the path of its API-socket is unique for
/// each virtual_machine.
///
/// # Arguments
/// * `socket_path` - Path of the API-socket of the cloud-hypervisor process
///
/// # Returns
/// * `Ok(())` if the process was killed or doesn't exist anymore
/// * `Err(AinariError)` if pkill could not be executed or failed
fn kill_vmm(socket_path: &str) -> Result<(), AinariError> {
    let pattern = format!("--api-socket {socket_path}");
    let status = Command::new("pkill")
        .args(["-KILL", "-f", "--", &pattern])
        .status()
        .map_err(|e| AinariError::InternalError(format!("Failed to execute pkill process: {e}")))?;

    // exit-code 1 of pkill means, that no process matched, so it is already gone
    match status.code() {
        Some(0) | Some(1) => Ok(()),
        _ => Err(AinariError::InternalError(format!(
            "pkill failed with exit status: {status}"
        ))),
    }
}

/// Removes a file, which is not required to exist
///
/// # Arguments
/// * `path` - Path of the file to remove
///
/// # Returns
/// * `Ok(())` if the file was removed or didn't exist
/// * `Err(AinariError)` if the file could not be removed
fn remove_file(path: &str) -> Result<(), AinariError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AinariError::InternalError(format!(
            "Failed to delete file {path}: {e}"
        ))),
    }
}

/// Removes a directory together with all of its content, which is not required to exist
///
/// # Arguments
/// * `path` - Path of the directory to remove
///
/// # Returns
/// * `Ok(())` if the directory was removed or didn't exist
/// * `Err(AinariError)` if the directory could not be removed
fn remove_directory(path: &str) -> Result<(), AinariError> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AinariError::InternalError(format!(
            "Failed to delete directory {path}: {e}"
        ))),
    }
}
