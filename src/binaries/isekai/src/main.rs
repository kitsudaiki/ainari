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

use core::processing::{jobs, worker_handler};

/// Entrypoint of isekai.
///
/// Orchestrates the backups and restores of single virtual_machines and whole projects. Isekai
/// doesn't store any data of the backups itself, it only collects the metadata of the other
/// components and lets them store and restore the files.
///
/// Sets up the logging, initializes the database and the workers of the jobs and then hands over
/// to the http-server, which blocks until the service is stopped.
///
/// # Returns
///
/// `Ok(())` after a clean shutdown, or the error, which made the startup fail.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = &config::CONFIG;
    ainari_common::logger::init_logger(config.log_type, &config.log_path, "isekai", config.debug)?;

    OmikujiLogging::new(
        "isekai",
        &config.miko.public(),
        &config::INTERNAL_API_KEY,
        config.skip_tls_verification,
    )
    .init();

    database::init_database()?;

    // the queues and the passphrases of the jobs didn't survive the restart
    jobs::fail_unfinished_jobs();

    lazy_static::initialize(&worker_handler::WORKER_HANDLER);

    api::http_server::run_server()?;

    Ok(())
}
