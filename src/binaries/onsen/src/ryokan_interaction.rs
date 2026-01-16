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
use tokio::task::LocalSet;

use crate::config;

use ainari_clients::endpoints::*;
use ainari_clients::host::register_onsen_host;
use ainari_common::error::AinariError;

/// Registers the current host with the Ainari Onsen service.
///
/// This function creates a local execution context and runs the host registration process.
/// It fetches endpoints from the Miko service, retrieves the local host name, and registers
/// the host with the Ryokan service using the provided configuration.
///
/// # Errors
///
/// Returns an `AinariError` if:
/// - The endpoints cannot be fetched from Miko
/// - The host name cannot be retrieved
/// - The registration with Ryokan fails
pub async fn register_host() -> Result<(), AinariError> {
    let local = LocalSet::new();

    local
        .run_until(async {
            // get endpoints from miko
            let miko_endpoint = &config::CONFIG.miko;
            // Fetch service endpoints from the Miko service
            // This includes addresses for various Ainari services
            let endpoints =
                get_endpoints(miko_endpoint, config::CONFIG.skip_tls_verification).await?;

            // Attempt to get the local host name
            let host_name = if let Some(host_name) = System::host_name() {
                host_name
            } else {
                return Err(AinariError::InternalError(
                    "Failed to get host-name".to_string(),
                ));
            };

            log::debug!("read host-name: {host_name}");

            // Register the current host with the Ryokan service
            // This includes providing authentication information and host details
            register_onsen_host(
                &endpoints.ryokan,
                &config::INTERNAL_API_KEY,
                &host_name,
                &config::CONFIG.address,
                &config::ONSEN_REGISTRATION_KEY,
                config::CONFIG.skip_tls_verification,
            )
            .await?;

            Ok::<_, AinariError>(())
        })
        .await?;

    Ok(())
}
