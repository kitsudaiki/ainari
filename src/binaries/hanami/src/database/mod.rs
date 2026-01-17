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

pub mod db_handle;
pub mod host_table;
pub mod meta_model_table;

/// Initializes all database tables required for the application.
///
/// This function orchestrates the initialization of all database tables
/// in the correct order. If any table fails to initialize, the entire
/// operation fails and returns an error.
///
/// # Returns
/// * `Ok(())` - All tables initialized successfully
/// * `Err(Box<dyn std::error::Error>)` - One or more tables failed to initialize
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize host-table
    match host_table::init_host_table() {
        Ok(_) => log::info!("Initialized host-database-table"),
        Err(e) => {
            log::error!("Failed to initialize host-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize meta model table
    match meta_model_table::init_meta_model_table() {
        Ok(_) => log::info!("Initialized model-database-table"),
        Err(e) => {
            log::error!("Failed to initialize model-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
}
