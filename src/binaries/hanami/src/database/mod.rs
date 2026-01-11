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
pub mod meta_cluster_table;

pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize host-table
    match host_table::init_host_table() {
        Ok(_) => log::info!("Initilaized host-database-table"),
        Err(e) => {
            log::error!("Failed to initialize host-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize host-table
    match meta_cluster_table::init_meta_cluster_table() {
        Ok(_) => log::info!("Initilaized cluster-database-table"),
        Err(e) => {
            log::error!("Failed to initialize cluster-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
}
