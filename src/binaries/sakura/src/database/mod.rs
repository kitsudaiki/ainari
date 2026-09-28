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

/// Opens the database of the service and applies all pending migrations of the
/// `migrations`-directory, which create and update the database-tables.
/// Afterwards all existing virtual_machines are removed from the database to ensure consistency
/// after a restart.
///
/// # Panics
///
/// Panics, if the database can not be opened or a migration fails, because the service can not
/// work with an incomplete database.
///
/// # Returns
///
/// * `Ok(())` - If all database operations complete successfully.
/// * `Err(Box<dyn std::error::Error>)` - If the virtual_machines could not be removed.
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Open the database and apply all pending migrations, which creates and updates the tables.
    // This is done explicitly here, so a broken database is already detected at startup.
    lazy_static::initialize(&db_handle::DB_CONN);
    log::info!("Applied all database-migrations");

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
