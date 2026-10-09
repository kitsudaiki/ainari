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

use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use super::super::delete_ch_virtual_machine::stop_vmm;
use super::super::start_ch_virtual_machine::start_ch_virtual_machine;
use super::super::{connect_to_vmm, set_vm_state, shutdown_gracefully, vm_socket_path, vmm_exited};
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::VirtualMachineState;

use cloud_hypervisor_client::models::VmState;

/// Prepares a virtual_machine on the source host for its migration
///
/// A running guest is shut down gracefully, so it writes all its data to its disk, and its
/// cloud-hypervisor process is stopped, because it holds a lock on the root-disk. Afterwards the
/// virtual_machine is frozen in the state `MIGRATING`, which blocks every other operation on it,
/// so its files don't change anymore, while the target pulls them.
///
/// A virtual_machine, which is already exported, is left as it is, so a repeated export doesn't
/// fail.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine is shut down and frozen
/// * `Err(AinariError::InvalidInput)` if the virtual_machine is in a state, which can't be
///   migrated, otherwise an appropriate error on failure
pub async fn export_ch_virtual_machine(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    let virtual_machine_data = virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;

    let state = virtual_machine_data.vm_state.as_str();
    if state == VirtualMachineState::Migrating.as_str() {
        log::warn!("VM {uuid} is already exported for its migration.");
        return Ok(());
    }
    if state != VirtualMachineState::Running.as_str()
        && state != VirtualMachineState::Stopped.as_str()
    {
        return Err(AinariError::InvalidInput(format!(
            "VM {uuid} is in state {state}, but only a running or stopped VM can be migrated."
        )));
    }
    if virtual_machine_data.root_disk_path.is_none() {
        return Err(AinariError::InvalidInput(format!(
            "VM {uuid} has no root-disk, which can be migrated."
        )));
    }

    log::info!("Export VM {uuid} for its migration");

    if !vmm_exited(uuid) {
        let (client, vm_state) = connect_to_vmm(uuid, context).await?;
        if matches!(vm_state, VmState::Running | VmState::Paused) {
            shutdown_gracefully(uuid, &client, vm_state).await?;
        }

        // cloud-hypervisor exits by itself, when the guest powered itself off, but keeps running
        // after a hard power-off
        let socket_path = vm_socket_path(uuid);
        if !vmm_exited(uuid) {
            stop_vmm(uuid, &socket_path).await?;
        }
        // a killed process leaves its socket behind, which would look like a running process
        let _ = fs::remove_file(&socket_path);
    }

    set_vm_state(uuid, VirtualMachineState::Migrating, context)?;

    log::info!("VM {uuid} exported for its migration");
    Ok(())
}

/// Unfreezes a virtual_machine on the source host, whose migration failed
///
/// The virtual_machine is marked as stopped again and booted, if it was running before the
/// migration. A virtual_machine, which is not frozen, is left as it is, so a cancellation after
/// a failed export doesn't change anything.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `boot` - Boot the virtual_machine again
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine is unfrozen and booted, if requested
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn cancel_ch_export(
    uuid: &Uuid,
    boot: bool,
    context: &UserContext,
) -> Result<(), AinariError> {
    let virtual_machine_data = virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;

    if virtual_machine_data.vm_state != VirtualMachineState::Migrating.as_str() {
        log::warn!("VM {uuid} is not exported, so there is no migration to cancel.");
        return Ok(());
    }

    log::info!("Cancel migration of VM {uuid}");
    set_vm_state(uuid, VirtualMachineState::Stopped, context)?;
    if boot {
        start_ch_virtual_machine(uuid, context).await?;
    }

    Ok(())
}
