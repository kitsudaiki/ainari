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

use std::error::Error;
use std::env;

mod api;
mod database;
mod config;

use log::{error, info};

use database::user_table::init_user_table;
use database::cluster_table::init_cluster_table;
use database::dataset_table::init_dataset_table;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger
    unsafe {
        // HINT (kitsudaiki): on my test-environment the rust-compile required the 'unsafe'-marker
        env::set_var("RUST_LOG", "debug");
    }
    
    env_logger::init(); 

    // Initialize user-table
    match init_user_table() {
        Ok(_) => info!("Initilaized user-datbase-table"),
        Err(e) => {
            error!("Failed to initialize user-database-table: {}", e);
            return Err(e);
        }
    };
    // Initialize cluster-table
    match init_cluster_table() {
        Ok(_) => info!("Initilaized cluster-datbase-table"),
        Err(e) => {
            error!("Failed to initialize cluster-database-table: {}", e);
            return Err(e);
        }
    };
    // Initialize dataset-table
    match init_dataset_table() {
        Ok(_) => info!("Initilaized dataset-datbase-table"),
        Err(e) => {
            error!("Failed to initialize dataset-database-table: {}", e);
            return Err(e);
        }
    };

    api::http_server::run_server()?;
    
    Ok(())
}
