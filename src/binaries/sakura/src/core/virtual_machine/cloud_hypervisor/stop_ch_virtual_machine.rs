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

use super::connect_to_vmm;

/// Shuts down a cloud-hypervisor virtual_machine
///
/// Only the virtual_machine is shut down, while its cloud-hypervisor process keeps running
/// together with the configuration of the virtual_machine, so it can be booted again later. A
/// virtual_machine, which is already shut down, is left untouched.
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to stop
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine is shut down
/// * `Err(AinariError)` with an appropriate error on failure
pub async fn stop_ch_virtual_machine(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    let (client, state) = connect_to_vmm(uuid, context).await?;

    if matches!(state, VmState::Created | VmState::Shutdown) {
        log::warn!("VM {uuid} is not running, so there is nothing to stop.");
        return Ok(());
    }

    log::info!("Stop VM {uuid}");
    client
        .shutdown_vm()
        .await
        .map_err(|e| AinariError::InternalError(format!("Shutdown VM {uuid} failed: {e:?}")))?;
    log::info!("VM {uuid} stopped");

    Ok(())
}
