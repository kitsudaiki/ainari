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

use sysinfo::System;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

use crate::config;
use crate::database::model_table;

use ainari_api::common_functions::convert_uuid;
use ainari_api_structs::host_structs::UuidList;
use ainari_clients::endpoints::*;
use ainari_clients::host::register_sakura_host;
use ainari_common::error::AinariError;

/// Registers the current host with the Ainari system.
///
/// This function:
/// 1. Creates a Tokio runtime for asynchronous operations
/// 2. Retrieves system endpoints from Miko
/// 3. Gathers information about the host system
/// 4. Collects UUIDs of deleted models from the database
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

    // Retrieve list of deleted models from the database
    let deleted_models = match model_table::list_deleted_models() {
        Ok(models) => models,
        Err(e) => {
            log::error!("Failed to get list of models form database: '{e}'");
            return Err(AinariError::InternalError("Internal Error".to_string()));
        }
    };

    // Prepare a list of UUIDs for deleted models
    let mut resp = UuidList { list: Vec::new() };

    // Convert each model UUID to the required format
    for model in deleted_models {
        let uuid = match convert_uuid(&model.uuid) {
            Ok(uuid) => uuid,
            Err(e) => {
                log::error!("Failed to convert UUID: '{e}'");
                return Err(AinariError::InternalError("Internal Error".to_string()));
            }
        };
        resp.list.push(uuid);
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
            config::CONFIG.skip_tls_verification,
        )
        .await
    })?;

    Ok(())
}
