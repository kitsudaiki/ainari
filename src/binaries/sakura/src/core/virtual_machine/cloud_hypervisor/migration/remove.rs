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

use uuid::Uuid;

use ainari_api::common_functions::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums::DbError;
use ainari_common::error::AinariError;

use super::super::delete_ch_virtual_machine::remove_vm_from_host;
use crate::database::virtual_machine_table;
use crate::database::virtual_machine_table::VirtualMachineState;

/// Removes the copy of a migrated virtual_machine from this host
///
/// This is the source after a successful migration, or the target after a failed one. Only a
/// virtual_machine, which is frozen for its migration or whose import failed, can be removed, so
/// a running virtual_machine is never removed by mistake. Unlike a normal deletion, no deleted
/// database-entry is kept (see `virtual_machine_table::remove_virtual_machine`).
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine is gone from this host or was never here
/// * `Err(AinariError::InvalidInput)` if the virtual_machine is not part of a migration,
///   otherwise an appropriate error on failure
pub async fn remove_migrated_ch_virtual_machine(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    let virtual_machine_data = match virtual_machine_table::get_virtual_machine(uuid, context) {
        Ok(virtual_machine_data) => virtual_machine_data,
        Err(DbError::NotFound) => {
            log::warn!("VM {uuid} is not on this host, so there is nothing to remove.");
            return Ok(());
        }
        Err(e) => {
            return Err(map_db_uuid_get_delete_ainari_error(
                "virtual_machine",
                uuid,
                e,
            ));
        }
    };

    let state = virtual_machine_data.vm_state.as_str();
    if state != VirtualMachineState::Migrating.as_str()
        && state != VirtualMachineState::Error.as_str()
    {
        return Err(AinariError::InvalidInput(format!(
            "VM {uuid} is in state {state} and not part of a migration, so it is not removed."
        )));
    }

    log::info!("Remove migrated VM {uuid} from this host");
    remove_vm_from_host(uuid).await?;
    virtual_machine_table::remove_virtual_machine(uuid)
        .map_err(|e| map_db_uuid_get_delete_ainari_error("virtual_machine", uuid, e))?;

    log::info!("Migrated VM {uuid} removed from this host");
    Ok(())
}
