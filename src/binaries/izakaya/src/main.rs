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

/// Entrypoint of the izakaya.
///
/// Shares the MLS key-packages of the gateways, delivers the MLS-messages between them and
/// coordinates the changes and key-rotations of their groups, so the gateways of a network can
/// agree on the keys, which protect the traffic between them.
///
/// Sets up the logging, initializes the database and then hands over to the http-server,
/// which blocks until the service is stopped.
///
/// # Returns
///
/// `Ok(())` after a clean shutdown, or the error, which made the startup fail.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = &config::CONFIG;
    ainari_common::logger::init_logger(config.log_type, &config.log_path, "izakaya", config.debug)?;

    database::init_database()?;

    // the public key of hanami is checked at the start, instead of with the first grant
    once_cell::sync::Lazy::force(&config::GRANT_PUBLIC_KEY);

    // rounds and operations, which stall, the grace-period of the rounds and the regular
    // key-rotation only depend on the time, so they are moved on in the background
    core::coordinator::spawn_ticker(
        config.mls.key_rotation_interval as i64,
        config.mls.member_timeout as i64,
    );

    api::http_server::run_server()?;

    Ok(())
}
