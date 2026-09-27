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

/// Entrypoint of the hanami.
///
/// Manages the sakura-hosts, the virtual networks and the floating ip-addresses.
///
/// Sets up the logging, initializes the database and then hands over to the http-server,
/// which blocks until the service is stopped.
///
/// # Returns
///
/// `Ok(())` after a clean shutdown, or the error, which made the startup fail.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = &config::CONFIG;
    ainari_common::logger::init_logger(config.log_type, &config.log_path, "hanami", config.debug)?;

    database::init_database()?;

    api::http_server::run_server()?;

    Ok(())
}
