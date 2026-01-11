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
use crate::database::cluster_table;

use ainari_api::common_functions::convert_uuid;
use ainari_api_structs::host_structs::UuidList;
use ainari_clients::endpoints::*;
use ainari_clients::host::register_sakura_host;
use ainari_common::error::AinariError;

pub fn register_host() -> Result<(), AinariError> {
    // Create a single-threaded runtime
    let rt = Builder::new_current_thread()
        .enable_all() // I/O & timers
        .build()
        .expect("failed to build runtime");

    // LocalSet allows spawn_local to work
    let local = LocalSet::new();

    // get endpoints from miko
    let miko_endpoint = &config::CONFIG.miko;
    let endpoints = local.block_on(&rt, async {
        get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification).await
    })?;

    let host_name = if let Some(host_name) = System::host_name() {
        host_name
    } else {
        return Err(AinariError::Error("Failed to get host-name".to_string()));
    };

    log::debug!("read host-name: {host_name}");

    let deleted_clusters = match cluster_table::list_deleted_clusters() {
        Ok(clusters) => clusters,
        Err(e) => {
            log::error!("Failed to get list of clusters form database: '{e}'");
            return Err(AinariError::Error("Internal Error".to_string()));
        }
    };

    let mut resp = UuidList { list: Vec::new() };

    for cluster in deleted_clusters {
        let uuid = match convert_uuid(&cluster.uuid) {
            Ok(uuid) => uuid,
            Err(e) => {
                log::error!("Failed to convert UUID: '{e}'");
                return Err(AinariError::Error("Internal Error".to_string()));
            }
        };
        resp.list.push(uuid);
    }

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
