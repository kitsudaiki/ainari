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
pub mod public_key_table;
pub mod secret_table;
pub mod simple_crypto_table;

/// Creates all database-tables of the service, if they not already exist.
///
/// Initializes the public-key-, secret- and crypto-tables in order. If one of them fails, the whole
/// initialization fails, because the service can not work with an incomplete database.
///
/// # Returns
///
/// * `Ok(())` - All tables are available.
/// * `Err(Box<dyn std::error::Error>)` - One of the tables could not be initialized.
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize host-table
    match secret_table::init_secret_table() {
        Ok(_) => log::info!("Initilaized secret-database-table"),
        Err(e) => {
            log::error!("Failed to initialize secret-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize host-table
    match simple_crypto_table::init_simple_crypto_table() {
        Ok(_) => log::info!("Initilaized simple-crypto-database-table"),
        Err(e) => {
            log::error!("Failed to initialize simple-crypto-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize public-key-table
    match public_key_table::init_public_key_table() {
        Ok(_) => log::info!("Initilaized public-key-database-table"),
        Err(e) => {
            log::error!("Failed to initialize public-key-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
}
