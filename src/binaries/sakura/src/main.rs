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

use log::LevelFilter;

use core::model_handler::*;
use core::processing::worker_handler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let enable_debug_log = config::CONFIG.debug;
    if !enable_debug_log {
        log::set_max_level(LevelFilter::Info);
    }

    // create directories if they not exist
    let tempfile_dir = config::CONFIG.storage.tempfile_location.clone();
    fs::create_dir_all(&tempfile_dir)?;
    let _ = clear_directory(&tempfile_dir);

    // Initialize processing
    let worker_handler = worker_handler::WORKER_HANDLER
        .lock()
        .expect("mutex poisoned");
    drop(worker_handler);
    let model_data_handler = CLUSTER_HANDLER.write().expect("mutex poisoned");
    drop(model_data_handler);

    database::init_database()?;

    hanami_interaction::register_host()?;

    api::http_server::run_server()?;

    Ok(())
}
