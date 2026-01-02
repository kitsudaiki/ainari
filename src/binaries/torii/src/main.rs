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

use log::LevelFilter;

use core::proxy_handler::*;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let enable_debug_log = config::CONFIG.debug;
    if !enable_debug_log {
        log::set_max_level(LevelFilter::Info);
    }

    database::init_database()?;

    let mut proxy_handler = PROXY_HANDLER.write().await;
    proxy_handler.fill_proxy_handler().await?;
    drop(proxy_handler);

    api::http_server::run_server().await?;

    Ok(())
}
