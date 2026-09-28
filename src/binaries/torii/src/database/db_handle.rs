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
    pub static ref DB_CONN: Arc<Mutex<SqliteConnection>> = Arc::new(Mutex::new(establish_connection()));
}

/// Opens the connection to the sqlite-database of the service and applies all pending
/// migrations to it.
///
/// This is called once to fill the `DB_CONN`-singleton, which is shared by all tables.
///
/// # Returns
///
/// The open connection.
///
/// # Panics
///
/// Panics, if the database-file can not be opened or the migrations can not be applied,
/// because the service can not work without its database.
pub fn establish_connection() -> SqliteConnection {
    let file_path = config::CONFIG.database.file_path.clone();
    //let database_url = ":memory:".to_string();
    let mut conn = SqliteConnection::establish(&file_path).expect("Error connecting to database");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Error applying the migrations to the database");
    conn
}
