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

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use std::sync::{Arc, Mutex};

use crate::config;

lazy_static::lazy_static! {
    /// The global database connection instance.
    ///
    /// # Safety
    /// This connection should only be accessed through proper synchronization mechanisms.
    /// Always use `lock()` to access the underlying connection.
    pub static ref DB_CONN: Arc<Mutex<SqliteConnection>> = Arc::new(Mutex::new(establish_connection()));
}

/// Establishes a new connection to the SQLite database.
///
/// This function reads the database file path from the application configuration
/// and attempts to establish a connection to it.
///
/// # Returns
///
/// * `SqliteConnection` - A new database connection
///
/// # Errors
///
/// This function will panic if it fails to establish a connection to the database.
/// In a production environment, you might want to handle this more gracefully.
pub fn establish_connection() -> SqliteConnection {
    let file_path = config::CONFIG.database.file_path.clone();
    // Alternative for in-memory database:
    // let database_url = ":memory:".to_string();
    SqliteConnection::establish(&file_path).expect("Error connecting to database")
}
