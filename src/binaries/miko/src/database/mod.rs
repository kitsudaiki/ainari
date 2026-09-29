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

/// Opens the database of the service and applies all pending migrations of the
/// `migrations`-directory, which create and update the database-tables.
/// Afterwards the initial admin-user and its quota are created, if the database is still empty.
///
/// # Panics
///
/// Panics, if the database can not be opened or a migration fails, because the service can not
/// work with an incomplete database.
///
/// # Returns
///
/// * `Ok(())` - The database is up to date and contains the initial admin.
/// * `Err(Box<dyn std::error::Error>)` - The initial admin could not be created.
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Open the database and apply all pending migrations, which creates and updates the tables.
    // This is done explicitly here, so a broken database is already detected at startup.
    lazy_static::initialize(&db_handle::DB_CONN);
    log::info!("Applied all database-migrations");

    // Multiple replicas of miko can start at the same time with the same database, so the
    // initial entries are created exclusively. Otherwise each of them would see the empty tables
    // and create its own admin.
    db_handle::DB_CONN.run_exclusively("init_admin", init_admin_entries)??;
    Ok(())
}

/// Creates the initial admin-user and its quota, if the tables are still empty.
///
/// # Returns
///
/// * `Ok(())` - The admin exists.
/// * `Err(Box<dyn std::error::Error>)` - The admin or its quota could not be created.
fn init_admin_entries() -> Result<(), Box<dyn std::error::Error>> {
    // Create the initial admin-user, if the user-table is still empty
    match user_table::init_admin() {
        Ok(_) => log::info!("Initialized admin-user"),
        Err(e) => {
            log::error!("Failed to initialize admin-user: {e}");
            return Err(e);
        }
    };

    // Create the quota of the initial admin, if the quota-table is still empty
    match quota_table::init_admin_quota() {
        Ok(_) => log::info!("Initialized admin-quota"),
        Err(e) => {
            log::error!("Failed to initialize admin-quota: {e}");
            return Err(e);
        }
    };
    Ok(())
}
