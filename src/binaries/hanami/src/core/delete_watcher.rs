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

use std::thread;
use std::time::Duration;

use uuid::Uuid;

use crate::config;
use crate::database::host_table::{self, HostResources};

use ainari_api_structs::user_context::UserContext;
use ainari_clients::virtual_machine as virtual_machine_clients;
use ainari_common::error::AinariError;

/// Time between two checks, if the virtual_machine is deleted on its sakura-host
const CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// Maximum number of checks, before the watcher gives up. The deletion-task on the sakura-host
/// can wait behind a creation-task of the same virtual_machine, so this is generous.
const MAX_NUMBER_OF_CHECKS: u32 = 3600;

/// Spawns a watcher-thread, which waits until a virtual_machine is deleted on its sakura-host
/// and releases the resources of the virtual_machine on this host afterwards
///
/// The sakura-host deletes the virtual_machine within a task in the background, so the resources
/// can not be released directly, when the delete-request was sent. The thread checks every
/// second, if the virtual_machine is gone, reduces the used values of the host in the
/// hosts-table and finishes itself afterwards.
///
/// # Arguments
/// * `virtual_machine_uuid` - UUID of the virtual_machine, which is deleted
/// * `host_uuid` - UUID of the sakura-host, which runs the virtual_machine
/// * `resources` - Cores, memory in MiB and disk-space in GiB of the virtual_machine
/// * `context` - User context, whose token is used to request the sakura-host
pub fn spawn_delete_watcher(
    virtual_machine_uuid: Uuid,
    host_uuid: Uuid,
    resources: HostResources,
    context: UserContext,
) {
    thread::spawn(move || {
        log::debug!("Started delete-watcher of VM {virtual_machine_uuid}.");

        // the http-client is bound to an actix-runtime, so the thread gets its own one
        let system = actix_rt::System::new();
        let is_deleted = system.block_on(wait_for_deletion(
            &virtual_machine_uuid,
            &host_uuid,
            &context,
        ));

        if !is_deleted {
            log::error!(
                "VM {virtual_machine_uuid} was not deleted on host {host_uuid} in time, so its \
                 resources are not released."
            );
            return;
        }

        match host_table::release_host_resources(&host_uuid, &resources) {
            Ok(()) => log::info!(
                "Released resources of deleted VM {virtual_machine_uuid} on host {host_uuid}: \
                 cores: {}, memory: {} MiB, disk: {} GiB.",
                resources.number_of_cores,
                resources.memory_size,
                resources.disk_space
            ),
            Err(_) => log::error!(
                "Failed to release resources of deleted VM {virtual_machine_uuid} on host \
                 {host_uuid}."
            ),
        }
    });
}

/// Checks every second, if a virtual_machine is deleted on its sakura-host
///
/// A failed check doesn't stop the watcher, because the sakura-host can be temporarily not
/// reachable.
///
/// # Arguments
/// * `virtual_machine_uuid` - UUID of the virtual_machine, which is deleted
/// * `host_uuid` - UUID of the sakura-host, which runs the virtual_machine
/// * `context` - User context, whose token is used to request the sakura-host
///
/// # Returns
/// * `true` if the virtual_machine is deleted
/// * `false` if it was still not deleted after the maximum number of checks
async fn wait_for_deletion(
    virtual_machine_uuid: &Uuid,
    host_uuid: &Uuid,
    context: &UserContext,
) -> bool {
    for _ in 0..MAX_NUMBER_OF_CHECKS {
        actix_rt::time::sleep(CHECK_INTERVAL).await;

        match is_deleted(virtual_machine_uuid, host_uuid, context).await {
            Ok(true) => return true,
            Ok(false) => {}
            Err(e) => log::warn!("Failed to check deletion of VM {virtual_machine_uuid}: {e}"),
        }
    }

    false
}

/// Checks once, if a virtual_machine is deleted on its sakura-host
///
/// # Arguments
/// * `virtual_machine_uuid` - UUID of the virtual_machine, which is deleted
/// * `host_uuid` - UUID of the sakura-host, which runs the virtual_machine
/// * `context` - User context, whose token is used to request the sakura-host
///
/// # Returns
/// * `Ok(bool)` with `true`, if the virtual_machine is deleted
/// * `Err(AinariError)` if the host or the virtual_machine could not be requested
async fn is_deleted(
    virtual_machine_uuid: &Uuid,
    host_uuid: &Uuid,
    context: &UserContext,
) -> Result<bool, AinariError> {
    let host_data = host_table::get_host(host_uuid, context).map_err(|_| {
        AinariError::InternalError(format!("Failed to get host {host_uuid} from database"))
    })?;

    virtual_machine_clients::is_virtual_machine_deleted(
        &host_data.address,
        &context.token,
        &config::INTERNAL_API_KEY,
        virtual_machine_uuid,
        config::CONFIG.skip_tls_verification,
    )
    .await
}
