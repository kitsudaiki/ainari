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

use std::path::Path;
use std::time::{Duration, Instant};

use cloud_hypervisor_client::SocketBasedApiClient;
use cloud_hypervisor_client::apis::DefaultApi;
use cloud_hypervisor_client::models::VmState;
use uuid::Uuid;

use ainari_common::error::AinariError;

use super::vm_socket_path;

/// Time, which the guest gets to shut itself down after the power-button was pressed, before it
/// is powered off hard
const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(60);

/// Interval, in which the state of the virtual_machine is checked while it shuts down
const SHUTDOWN_POLL_INTERVAL: Duration = Duration::from_millis(500);

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
