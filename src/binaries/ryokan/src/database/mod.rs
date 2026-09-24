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
pub mod image_table;
pub mod snapshot_table;

/// Creates all database-tables of the service, if they not already exist.
///
/// Initializes the host-, image- and snapshot-tables in order. If one of them fails, the whole
/// initialization fails, because the service can not work with an incomplete database.
///
/// # Returns
///
/// * `Ok(())` - All tables are available.
/// * `Err(Box<dyn std::error::Error>)` - One of the tables could not be initialized.
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize host-table
    match host_table::init_host_table() {
        Ok(_) => log::info!("Initilaized host-database-table"),
        Err(e) => {
            log::error!("Failed to initialize host-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize image-table
    match image_table::init_image_table() {
        Ok(_) => log::info!("Initilaized image-database-table"),
        Err(e) => {
            log::error!("Failed to initialize image-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize snapshot-table
    match snapshot_table::init_snapshot_table() {
        Ok(_) => log::info!("Initilaized snapshot-database-table"),
        Err(e) => {
            log::error!("Failed to initialize snapshot-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
}
