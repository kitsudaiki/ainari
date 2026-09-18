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

pub mod address_table;
pub mod db_handle;
pub mod floating_ip_table;
pub mod host_table;
pub mod meta_virtual_machine_table;
pub mod network_table;

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

    // Initialize meta virtual_machine table
    match meta_virtual_machine_table::init_meta_virtual_machine_table() {
        Ok(_) => log::info!("Initialized virtual_machine-database-table"),
        Err(e) => {
            log::error!("Failed to initialize virtual_machine-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize network table
    match network_table::init_network_table() {
        Ok(_) => log::info!("Initialized network-database-table"),
        Err(e) => {
            log::error!("Failed to initialize network-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize floating-ip table
    match floating_ip_table::init_floating_ip_table() {
        Ok(_) => log::info!("Initialized floating-ip-database-table"),
        Err(e) => {
            log::error!("Failed to initialize floating-ip-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize address table
    match address_table::init_address_table() {
        Ok(_) => log::info!("Initialized address-database-table"),
        Err(e) => {
            log::error!("Failed to initialize address-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
}
