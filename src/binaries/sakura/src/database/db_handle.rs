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
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::sync::{Arc, Mutex};

use crate::config;

/// All migrations of the database of the service, which are embedded into the binary at
/// compile-time from the `migrations`-directory of the crate.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

lazy_static::lazy_static! {
    /// The global database connection virtual_machine.
    ///
    /// # Safety
    /// This connection should only be accessed through proper synchronization mechanisms.
    /// Always use `lock()` to access the underlying connection.
    pub static ref DB_CONN: Arc<Mutex<SqliteConnection>> = Arc::new(Mutex::new(establish_connection()));
}

/// Establishes a new connection to the SQLite database.
///
/// This function reads the database file path from the application configuration,
/// attempts to establish a connection to it and applies all pending migrations.
///
/// # Returns
///
/// * `SqliteConnection` - A new database connection
///
/// # Errors
///
/// This function will panic if it fails to establish a connection to the database or to
/// apply the migrations.
/// In a production environment, you might want to handle this more gracefully.
pub fn establish_connection() -> SqliteConnection {
    let file_path = config::CONFIG.database.file_path.clone();
    // Alternative for in-memory database:
    // let database_url = ":memory:".to_string();
    let mut conn = SqliteConnection::establish(&file_path).expect("Error connecting to database");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Error applying the migrations to the database");
    conn
}
