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
pub mod model_table;
pub mod task_table;

use std::io;

use ainari_common::enums;

pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize model-table
    match model_table::init_model_table() {
        Ok(_) => log::info!("Initilaized model-database-table"),
        Err(e) => {
            log::error!("Failed to initialize model-database-table: {e}");
            return Err(e);
        }
    };
    // Initialize task-table
    match task_table::init_task_table() {
        Ok(_) => log::info!("Initilaized task-database-table"),
        Err(e) => {
            log::error!("Failed to initialize task-database-table: {e}");
            return Err(e);
        }
    };

    // clear all model from the database. This is necessary, because after a restart,
    // all model are broken and so the database doesn't match the real world.
    // To "fix" this issue, all model have to be removed from the database as well.
    match model_table::delete_all_model() {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            let msg = "Error while deleting all model from DB".to_string();
            log::error!("{msg}");
            let error = io::Error::other(msg);
            return Err(Box::new(error));
        }
        Err(enums::DbError::NotFound) => {
            let error = io::Error::other("".to_string());
            return Err(Box::new(error));
        }
    }

    Ok(())
}
