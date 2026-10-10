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
mod database;

use ainari_api::omikuji_logging::OmikujiLogging;
use ainari_common::config::MikoEndpoint;

/// Entrypoint of the miko.
///
/// Provides the authentication together with the user-, project- and quota-management.
///
/// Sets up the logging, initializes the database and then hands over to the http-server,
/// which blocks until the service is stopped.
///
/// # Returns
///
/// `Ok(())` after a clean shutdown, or the error, which made the startup fail.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = &config::CONFIG;
    ainari_common::logger::init_logger(config.log_type, &config.log_path, "miko", config.debug)?;

    // miko can not request the address of omikuji from itself, so it is taken from its config
    OmikujiLogging::new(
        "miko",
        &MikoEndpoint::default(),
        &config::INTERNAL_API_KEY,
        config.skip_tls_verification,
    )
    .with_omikuji_endpoint(&config.endpoints.omikuji)
    .init();

    database::init_database()?;

    api::http_server::run_server()?;

    Ok(())
}
