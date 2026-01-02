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

pub mod checkpoint_table;
pub mod dataset_table;
pub mod db_handle;
pub mod host_table;

pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize host-table
    match host_table::init_host_table() {
        Ok(_) => log::info!("Initilaized host-database-table"),
        Err(e) => {
            log::error!("Failed to initialize host-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize dataset-table
    match dataset_table::init_dataset_table() {
        Ok(_) => log::info!("Initilaized dataset-database-table"),
        Err(e) => {
            log::error!("Failed to initialize dataset-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize checkpoint-table
    match checkpoint_table::init_checkpoint_table() {
        Ok(_) => log::info!("Initilaized checkpoint-database-table"),
        Err(e) => {
            log::error!("Failed to initialize checkpoint-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
}
