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

mod api;
mod database;
mod config;
mod core;

use log::{error, info};
use log::LevelFilter;

use core::cluster_handler;

autocxx::include_cpp! {
    #include "hanami_root.h"
    #include "hanami_structs.h"
    #include "cluster_link.h"
    safety!(unsafe_ffi)
    generate!("HanamiCore")
    generate!("ReturnStatus")
    generate!("createRootObj")
    generate!("ClusterMeta")
    generate!("ClusterLink")
}


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

    // Initialize hanami-core
    let use_of_free_memory: f32 = config::CONFIG.processing.use_of_free_memory.clone();
    let mut cluster_handle = cluster_handler::CLUSTER_HANDLER.lock().unwrap();
    if cluster_handle.init_hanami_root(use_of_free_memory) {
        info!("Initilaized hanami-core")
    } else {
        let msg = "Failed to initialize hanami-core".to_string();
        error!("{}", msg);
        return Err(msg.into());
    }
    drop(cluster_handle);

    database::init_database()?;

    api::http_server::run_server()?;
    
    Ok(())
}
