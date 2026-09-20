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
pub mod project_table;
pub mod quota_table;
pub mod user_table;

/// Creates all database-tables of the service, if they not already exist.
///
/// Initializes the user-, project- and quota-tables in order. If one of them fails, the whole
/// initialization fails, because the service can not work with an incomplete database.
///
/// # Returns
///
/// * `Ok(())` - All tables are available.
/// * `Err(Box<dyn std::error::Error>)` - One of the tables could not be initialized.
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize user-table
    match user_table::init_user_table() {
        Ok(_) => log::info!("Initilaized user-database-table"),
        Err(e) => {
            log::error!("Failed to initialize user-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize project-table
    match project_table::init_project_table() {
        Ok(_) => log::info!("Initilaized project-database-table"),
        Err(e) => {
            log::error!("Failed to initialize project-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize quota-table
    match quota_table::init_quota_table() {
        Ok(_) => log::info!("Initilaized quota-database-table"),
        Err(e) => {
            log::error!("Failed to initialize quota-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
}
