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
pub mod proxy_table;

use std::io;

use ainari_common::enums;

pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize proxy-table
    match proxy_table::init_proxy_table() {
        Ok(_) => log::info!("Initilaized proxy-database-table"),
        Err(e) => {
            log::error!("Failed to initialize proxy-database-table: {e}");
            return Err(e);
        }
    };

    // clear all proxy from the database. This is necessary, because after a restart,
    // all proxy are broken and so the database doesn't match the real world.
    // To "fix" this issue, all proxy have to be removed from the database as well.
    match proxy_table::delete_all_proxy() {
        Ok(_) => {}
        Err(enums::DbError::InternalError) => {
            let msg = "Error while deleting all proxy from DB".to_string();
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
