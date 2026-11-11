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

use crate::config;
use crate::core::backend::{Backend, VolumeBackend};

use ainari_clients::endpoints::*;
use ainari_clients::host::register_onsen_host;
use ainari_common::error::AinariError;

/// Registers the current host with the Ainari Onsen service.
///
/// It fetches endpoints from the Miko service, retrieves the local host name, and registers
/// the host with the Ryokan service using the provided configuration. The address of the internal
/// api of this host is registered, over which the ryokan and the sakura-hosts reach it, together
/// with the type of its storage-backend and its support of volumes.
///
/// # Arguments
///
/// * `backend` - Storage-backend of this host
///
/// # Errors
///
/// Returns an `AinariError` if:
/// - The endpoints cannot be fetched from Miko
/// - The host name cannot be retrieved
/// - The registration with Ryokan fails
pub async fn register_host(backend: &Backend) -> Result<(), AinariError> {
    // Fetch service endpoints from the Miko service
    // This includes addresses for various Ainari services
    let endpoints =
        get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification).await?;

    // Attempt to get the local host name
    let Some(host_name) = System::host_name() else {
        return Err(AinariError::internal("Failed to get host-name".to_string()));
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
        backend.backend_type(),
        backend.supports_volumes(),
        config::CONFIG.skip_tls_verification,
    )
    .await?;

    Ok(())
}
