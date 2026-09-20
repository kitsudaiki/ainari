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

mod api;
mod config;
mod core;
mod database;

use log::LevelFilter;

use core::proxy_handler::*;
use core::routing_interface::*;

/// Entrypoint of the torii.
///
/// Gateway with its routes, proxies, packet-filters and NAT-configuration.
///
/// Sets up the logging, initializes the database and then hands over to the http-server,
/// which blocks until the service is stopped.
///
/// The proxy-handler and the routing-state are restored from the database on startup, so the
/// gateway serves the connections, which already existed before the restart.
///
/// # Returns
///
/// `Ok(())` after a clean shutdown, or the error, which made the startup fail.
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

    let route_handler = GATEWAY_STATE_HANDLE.lock().await;
    drop(route_handler);

    api::http_server::run_server().await?;

    Ok(())
}
