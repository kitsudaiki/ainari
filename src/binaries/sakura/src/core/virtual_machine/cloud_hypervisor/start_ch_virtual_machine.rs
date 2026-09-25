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
use cloud_hypervisor_client::models::VmState;
use uuid::Uuid;

use ainari_api_structs::user_context::UserContext;
use ainari_common::error::AinariError;

use super::{connect_to_vmm, mark_error_on_failure, set_vm_state};
use crate::database::virtual_machine_table::VirtualMachineState;

/// Boots a stopped cloud-hypervisor virtual_machine again
///
/// The cloud-hypervisor process keeps the configuration of the virtual_machine while it is shut
/// down, so it is booted with the same disks and network as before. A virtual_machine, which
/// already runs, is left untouched. The virtual_machine is marked as running afterwards, or as
/// error, if it could not be started.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to start
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine runs
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn start_ch_virtual_machine(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    let result = start_vm(uuid, context).await;
    mark_error_on_failure(uuid, context, result)
}

/// Boots or resumes the virtual_machine, depending on its current state
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to start
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine runs
/// * `Err(AinariError)` with an appropriate error on failure
async fn start_vm(uuid: &Uuid, context: &UserContext) -> Result<(), AinariError> {
    let (client, state) = connect_to_vmm(uuid, context).await?;

    match state {
        VmState::Running => {
            log::warn!("VM {uuid} is already running, so there is nothing to start.");
        }
        // a paused virtual_machine is still booted, so it only has to continue
        VmState::Paused => {
            log::info!("Resume paused VM {uuid}");
            client.resume_vm().await.map_err(|e| {
                AinariError::InternalError(format!("Resume VM {uuid} failed: {e:?}"))
            })?;
        }
        VmState::Created | VmState::Shutdown => {
            log::info!("Start VM {uuid}");
            client
                .boot_vm()
                .await
                .map_err(|e| AinariError::InternalError(format!("Boot VM {uuid} failed: {e:?}")))?;
            log::info!("VM {uuid} started");
        }
    }

    set_vm_state(uuid, VirtualMachineState::Running, context)
}
