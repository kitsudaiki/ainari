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
use sysinfo::System;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

use crate::config;
use crate::database::virtual_machine_table;

use ainari_api_structs::host_structs::UuidList;
use ainari_clients::endpoints::*;
use ainari_clients::host::register_sakura_host;
use ainari_common::error::AinariError;
use ainari_hardware::cpu::get_number_of_cpu_threads;
use ainari_hardware::disk::get_total_disk_space;
use ainari_hardware::memory::get_total_memory_amount;

/// Registers the current host with the Ainari system.
///
/// This function:
/// 1. Creates a Tokio runtime for asynchronous operations
/// 2. Retrieves system endpoints from Miko
/// 3. Gathers information about the host system (name, cpu-threads, memory and disk-space)
/// 4. Collects UUIDs of deleted virtual_machines from the database
/// 5. Registers the host with Hanami using the collected information
///
/// # Errors
/// Returns an `AinariError` if any step in the registration process fails.
pub fn register_host() -> Result<(), AinariError> {
    // Create a single-threaded runtime for synchronous execution of async code
    let rt = Builder::new_current_thread()
        .enable_all() // Enable I/O and timers for the runtime
        .build()
        .expect("failed to build runtime");

    // LocalSet allows spawn_local to work, enabling local task execution
    let local = LocalSet::new();

    // Retrieve system endpoints from Miko service
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = local.block_on(&rt, async {
        get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification).await
    })?;

    // Get the host name from the system information
    let host_name = if let Some(host_name) = System::host_name() {
        host_name
    } else {
        return Err(AinariError::InternalError(
            "Failed to get host-name".to_string(),
        ));
    };

    log::debug!("read host-name: {host_name}");

    // Get the hardware-resources of the host
    let number_of_cores = get_number_of_cpu_threads().map_err(|e| {
        log::error!("Failed to read number of cpu-threads: '{e}'");
        AinariError::InternalError("Internal Error".to_string())
    })? as u64;
    let memory_size = get_total_memory_amount() / (1024 * 1024);
    let storage_path = &config::CONFIG.storage.local_vm_storage_path;
    let disk_space = get_total_disk_space(Path::new(storage_path)).map_err(|e| {
        log::error!("Failed to read disk-space for path '{storage_path}': '{e}'");
        AinariError::InternalError("Internal Error".to_string())
    })? / (1024 * 1024 * 1024);

    log::debug!(
        "read hardware: cpu-threads: {number_of_cores}, memory: {memory_size} MiB, disk: {disk_space} GiB"
    );

    // only the resources, which are not reserved for the host itself, are available for
    // virtual machines
    let host_config = &config::CONFIG.host;
    let number_of_cores =
        subtract_reserved("cpu-threads", number_of_cores, host_config.reserved_cores);
    let memory_size = subtract_reserved("memory", memory_size, host_config.reserved_memory);
    let disk_space = subtract_reserved("disk-space", disk_space, host_config.reserved_disk);

    log::debug!(
        "available for virtual machines: cpu-threads: {number_of_cores}, memory: {memory_size} MiB, disk: {disk_space} GiB"
    );

    // Retrieve list of deleted virtual_machines from the database
    let deleted_virtual_machines = match virtual_machine_table::list_deleted_virtual_machines() {
        Ok(virtual_machines) => virtual_machines,
        Err(e) => {
            log::error!("Failed to get list of virtual_machines form database: '{e}'");
            return Err(AinariError::InternalError("Internal Error".to_string()));
        }
    };

    // Prepare a list of UUIDs for deleted virtual_machines
    let mut resp = UuidList { list: Vec::new() };

    // Convert each virtual_machine UUID to the required format
    for virtual_machine in deleted_virtual_machines {
        resp.list.push(virtual_machine.uuid);
    }

    // Register the host with Hanami service
    local.block_on(&rt, async {
        register_sakura_host(
            &endpoints.hanami,
            &config::INTERNAL_API_KEY,
            &host_name,
            &config::CONFIG.address,
            resp,
            &config::SAKURA_REGISTRATION_KEY,
            number_of_cores,
            memory_size,
            disk_space,
            config::CONFIG.skip_tls_verification,
        )
        .await
    })?;

    Ok(())
}

/// Subtracts the reserved amount of a resource from the total amount of the host.
///
/// If more is reserved than the host has, nothing is left for virtual machines, so 0 is returned
/// and a warning is logged.
///
/// # Arguments
/// * `name` - Name of the resource for the log-message
/// * `total` - Total amount of the resource of the host
/// * `reserved` - Amount of the resource, which is reserved for the host
///
/// # Returns
/// The amount of the resource, which is available for virtual machines.
fn subtract_reserved(name: &str, total: u64, reserved: u64) -> u64 {
    if reserved > total {
        log::warn!(
            "Reserved {name} ({reserved}) is bigger than the {name} of the host ({total}), \
             so no {name} is available for virtual machines."
        );
    }
    total.saturating_sub(reserved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtract_reserved() {
        assert_eq!(subtract_reserved("cores", 16, 2), 14);
        assert_eq!(subtract_reserved("cores", 16, 0), 16);
        assert_eq!(subtract_reserved("cores", 16, 16), 0);
        assert_eq!(subtract_reserved("cores", 16, 20), 0);
    }
}
