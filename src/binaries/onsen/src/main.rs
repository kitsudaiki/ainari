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

#![forbid(unsafe_code)]

mod api;
mod config;
mod core;

use ainari_api::omikuji_logging::OmikujiLogging;

/// Entrypoint of the onsen.
///
/// Storage-pool, which holds the payloads of the images and snapshots. It is not reachable from
/// the internet and only provides internal endpoints, over which the ryokan and the sakura-hosts
/// store and read the files.
///
/// Prepares the storage-backend, which is selected by the config, registers this host on the
/// ryokan and then hands over to the http-server, which blocks until the service is stopped.
///
/// # Returns
///
/// `Ok(())` after a clean shutdown, or the error, which made the startup fail.
#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = &config::CONFIG;
    ainari_common::logger::init_logger(config.log_type, &config.log_path, "onsen", config.debug)?;

    OmikujiLogging::new(
        "onsen",
        &config.miko,
        &config::INTERNAL_API_KEY,
        config.skip_tls_verification,
    )
    .init();

    if config.storage.uses_legacy_location {
        log::warn!(
            "'storage.location' is deprecated and will be removed in a future release, use \
            'backend = \"local\"' and the section [storage.local] instead"
        );
    }
    let backend = core::backend::Backend::new(&config.storage.backend).await?;

    core::ryokan_interaction::register_host().await?;

    api::http_server::run_server(backend).await?;

    Ok(())
}
