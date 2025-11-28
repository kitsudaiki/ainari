// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
use tokio::task::LocalSet;

use crate::config;

use ainari_clients::endpoints::*;
use ainari_clients::host::register_onsen_host;
use ainari_common::error::AinariError;

pub async fn register_host() -> Result<(), AinariError> {
    let local = LocalSet::new();

    local
        .run_until(async {
            // get endpoints from miko
            let miko_endpoint = &config::CONFIG.miko;
            let endpoints =
                get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification).await?;

            let host_name = if let Some(host_name) = System::host_name() {
                host_name
            } else {
                return Err(AinariError::Error("Failed to get host-name".to_string()));
            };

            log::debug!("read host-name: {host_name}");

            register_onsen_host(
                &endpoints.ryokan,
                &config::CONFIG.api.internal_api_key,
                &host_name,
                &config::CONFIG.address,
                &config::CONFIG.ryokan.registation_key,
                config::CONFIG.skip_tls_verification,
            )
            .await?;

            Ok::<_, AinariError>(())
        })
        .await?;

    Ok(())
}
