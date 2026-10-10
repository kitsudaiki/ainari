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
mod hanami_interaction;

use ainari_common::functions::clear_directory;
use std::fs;

use core::processing::worker_handler;

/// Entrypoint of the sakura.
///
/// Runs and supervises the virtual machines of a single host.
///
/// Sets up the logging, initializes the database and then hands over to the http-server,
/// which blocks until the service is stopped.
///
/// The temporary directory is cleared and the worker-handler is initialized on startup, so
/// leftovers of a previous run are removed before new tasks are processed.
///
/// # Returns
///
/// `Ok(())` after a clean shutdown, or the error, which made the startup fail.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = &config::CONFIG;
    ainari_common::logger::init_logger(config.log_type, &config.log_path, "sakura", config.debug)?;

    // create directories if they not exist
    let tempfile_dir = config::CONFIG.storage.tempfile_location.clone();
    fs::create_dir_all(&tempfile_dir)?;
    let _ = clear_directory(&tempfile_dir);

    // Initialize processing
    let worker_handler = worker_handler::WORKER_HANDLER
        .lock()
        .expect("mutex poisoned");
    drop(worker_handler);

    database::init_database()?;

    // the task-queues didn't survive the restart, so their tasks are never processed
    match database::task_table::fail_unfinished_tasks("Failed because of a restart of the host.") {
        Ok(0) => {}
        Ok(count) => log::warn!("Marked {count} unfinished task(s) as failed after the restart"),
        Err(e) => log::error!("Failed to mark the unfinished tasks as failed: {e}"),
    }

    hanami_interaction::register_host()?;

    // the virtual_machines, which were running, ended together with sakura. Hanami restores
    // their network with the registration above.
    core::virtual_machine::cloud_hypervisor::restart_after_host_restart::spawn_restart_of_virtual_machines();

    api::http_server::run_server()?;

    Ok(())
}
