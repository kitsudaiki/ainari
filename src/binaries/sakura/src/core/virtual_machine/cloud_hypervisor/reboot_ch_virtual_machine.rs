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

/// Reboots a running cloud-hypervisor virtual_machine
///
/// # Arguments
/// * `uuid` - Unique identifier of the virtual_machine to reboot
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the virtual_machine was rebooted
/// * `Err(AinariError)` if the virtual_machine doesn't run or the reboot failed
pub async fn reboot_ch_virtual_machine(
    uuid: &Uuid,
    context: &UserContext,
) -> Result<(), AinariError> {
    let (client, state) = connect_to_vmm(uuid, context).await?;

    // cloud-hypervisor can only reboot a booted virtual_machine
    if !matches!(state, VmState::Running | VmState::Paused) {
        return Err(AinariError::InvalidInput(format!(
            "VM {uuid} is not running, so it can not be rebooted. Start it instead."
        )));
    }

    log::info!("Reboot VM {uuid}");
    client
        .reboot_vm()
        .await
        .map_err(|e| AinariError::InternalError(format!("Reboot VM {uuid} failed: {e:?}")))?;
    log::info!("VM {uuid} rebooted");

    Ok(())
}
