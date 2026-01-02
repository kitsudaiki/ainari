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

mod config;
mod ryokan_interaction;
mod server;

use log::LevelFilter;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let enable_debug_log = config::CONFIG.debug;
    if !enable_debug_log {
        log::set_max_level(LevelFilter::Info);
    }

    // create directories if they not exist
    let location = config::CONFIG.storage.location.clone();
    fs::create_dir_all(location)?;

    ryokan_interaction::register_host().await?;

    server::run_server().await?;

    Ok(())
}
