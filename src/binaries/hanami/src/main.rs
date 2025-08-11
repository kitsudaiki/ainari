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

use std::env;
use std::fs;

mod api;
mod config;
mod core;
mod database;

use log::LevelFilter;

use core::cluster_handler::*;
use core::processing::worker_handler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    unsafe {
        // HINT (kitsudaiki): on my test-environment the rust-compile required the 'unsafe'-marker
        env::set_var("RUST_LOG", "debug");
    }

    env_logger::init();

    let enable_debug_log = config::CONFIG.debug;
    if enable_debug_log == false {
        log::set_max_level(LevelFilter::Info);
    }

    // create directories if they not exist
    let checkpoint_dir = config::CONFIG.storage.checkpoint_location.clone();
    fs::create_dir_all(checkpoint_dir)?;
    let dataset_dir = config::CONFIG.storage.dataset_location.clone();
    fs::create_dir_all(dataset_dir)?;
    let tempfile_dir = config::CONFIG.storage.tempfile_location.clone();
    fs::create_dir_all(tempfile_dir)?;

    // Initialize processing
    let worker_handler = worker_handler::WORKER_HANDLER.lock().unwrap();
    drop(worker_handler);
    let cluster_data_handler = CLUSTER_HANDLER.write().unwrap();
    drop(cluster_data_handler);

    database::init_database()?;

    api::http_server::run_server()?;

    Ok(())
}
