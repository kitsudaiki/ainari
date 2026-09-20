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
pub mod task_table;
pub mod virtual_machine_table;

use std::io;

use ainari_common::enums;

/// Initializes the database by setting up required tables and clearing existing virtual_machine data.
///
/// This function performs several critical operations:
/// 1. Initializes the virtual_machine table in the database.
/// 2. Initializes the task table in the database.
/// 3. Clears all existing virtual_machine data from the database to ensure consistency after a restart.
///
/// # Returns
///
/// * `Ok(())` - If all database operations complete successfully.
/// * `Err(Box<dyn std::error::Error>)` - If any database operation fails.
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize virtual_machine-table
    match virtual_machine_table::init_virtual_machine_table() {
        Ok(_) => log::info!("Initialized virtual_machine-database-table"),
        Err(e) => {
            log::error!("Failed to initialize virtual_machine-database-table: {e}");
            return Err(e);
        }
    };
    // Initialize task-table
    match task_table::init_task_table() {
        Ok(_) => log::info!("Initialized task-database-table"),
        Err(e) => {
            log::error!("Failed to initialize task-database-table: {e}");
            return Err(e);
        }
    };

    // Clear all virtual_machine from the database. This is necessary because after a restart,
    // all virtual_machines are broken and the database doesn't match the real world.
    // To "fix" this issue, all virtual_machines have to be removed from the database as well.
    match virtual_machine_table::delete_all_virtual_machine() {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            let msg = "Error while deleting all virtual_machine from DB".to_string();
            log::error!("{msg}");
            let error = io::Error::other(msg);
            return Err(Box::new(error));
        }
        Err(enums::DbError::NotFound) => {
            // Treat NotFound as a recoverable error by returning an empty error message
            let error = io::Error::other("".to_string());
            return Err(Box::new(error));
        }
    }

    Ok(())
}
