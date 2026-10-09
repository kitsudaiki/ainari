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

//! Cold migration of a cloud-hypervisor virtual_machine to another sakura-host.
//!
//! hanami orchestrates the migration, this host only provides its steps, which run as tasks of
//! the virtual_machine, so they are serialized with all its other tasks:
//!
//! 1. `export`: the source shuts the virtual_machine down and freezes it in the state
//!    `MIGRATING`, so nothing changes its disk anymore.
//! 2. `import`: the target creates the virtual_machine with the same identity, pulls its
//!    seed-image and its root-disk from the source (see `transfer`) and boots it, if it was
//!    running before.
//! 3. `remove`: the source removes its copy after a successful migration, or the target after a
//!    failed one. `cancel` unfreezes the virtual_machine on the source again after a failure.
//!
//! The virtual_machine keeps its UUID, its address, its MAC-address and its TAP-device on the new
//! host. Only the network of the gateways is moved by hanami.

pub mod export;
pub mod import;
pub mod remove;
pub mod transfer;

use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::VirtualMachineState;

/// Rejects an operation on a virtual_machine, which is migrated right now.
///
/// A migrating virtual_machine must not be started or changed, because it is frozen on the source
/// and not complete yet on the target.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine is not migrated
/// * `Err(AinariError::Conflict)` if it is migrated, or another error if it can not be read
pub fn ensure_not_migrating(uuid: &Uuid, context: &UserContext) -> Result<(), AinariError> {
    let virtual_machine_data = virtual_machine_table::get_virtual_machine(uuid, context)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;

    if virtual_machine_data.vm_state == VirtualMachineState::Migrating.as_str() {
        return Err(AinariError::Conflict(format!(
            "VM {uuid} is migrated to another host right now."
        )));
    }
    Ok(())
}
