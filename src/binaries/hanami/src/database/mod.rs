// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
pub mod user_table;
pub mod cluster_table;
pub mod dataset_table;
pub mod task_table;
pub mod project_table;
pub mod checkpoint_table;

use std::io;

use ainari_common::enums;

pub fn init_database() -> Result<(), Box<dyn std::error::Error>>
{
    // Initialize user-table
    match user_table::init_user_table() {
        Ok(_) => log::info!("Initilaized user-database-table"),
        Err(e) => {
            log::error!("Failed to initialize user-database-table: {}", e);
            return Err(e);
        }
    };
    // Initialize project-table
    match project_table::init_project_table() {
        Ok(_) => log::info!("Initilaized project-database-table"),
        Err(e) => {
            log::error!("Failed to initialize project-database-table: {}", e);
            return Err(e);
        }
    };
    // Initialize cluster-table
    match cluster_table::init_cluster_table() {
        Ok(_) => log::info!("Initilaized cluster-database-table"),
        Err(e) => {
            log::error!("Failed to initialize cluster-database-table: {}", e);
            return Err(e);
        }
    };
    // Initialize dataset-table
    match dataset_table::init_dataset_table() {
        Ok(_) => log::info!("Initilaized dataset-database-table"),
        Err(e) => {
            log::error!("Failed to initialize dataset-database-table: {}", e);
            return Err(e);
        }
    };
    // Initialize task-table
    match task_table::init_task_table() {
        Ok(_) => log::info!("Initilaized task-database-table"),
        Err(e) => {
            log::error!("Failed to initialize task-database-table: {}", e);
            return Err(e);
        }
    };
    // Initialize checkpoint-table
    match checkpoint_table::init_checkpoint_table() {
        Ok(_) => log::info!("Initilaized checkpoint-database-table"),
        Err(e) => {
            log::error!("Failed to initialize checkpoint-database-table: {}", e);
            return Err(e);
        }
    };

    // clear all cluster from the database. This is necessary, because after a restart,
    // all cluster are broken and so the database doesn't match the real world.
    // To "fix" this issue, all cluster have to be removed from the database as well.
    match cluster_table::delete_all_cluster() {
        Ok(_) => {},
        Err(enums::DbError::InternalError) => {
            let msg = format!("Error while deleting all cluster from DB");
            log::error!("{msg}");
            let error = io::Error::new(io::ErrorKind::Other, msg);
            return Err(Box::new(error));
        },
        Err(enums::DbError::NotFound) => {
            let error = io::Error::new(io::ErrorKind::Other, "".to_string());
            return Err(Box::new(error));
        }
    }

    Ok(())
}
