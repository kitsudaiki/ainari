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

use diesel_migrations::{EmbeddedMigrations, embed_migrations};

use ainari_common::config::DatabaseConfig;
use ainari_common::database::{DbHandle, DbMigrations};

use crate::config;

/// All migrations of the sqlite-database of the service, which are embedded into the binary at
/// compile-time from the `migrations/sqlite`-directory of the crate.
const SQLITE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/sqlite");

/// All migrations of the mysql-database of the service, which are embedded into the binary at
/// compile-time from the `migrations/mysql`-directory of the crate.
const MYSQL_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/mysql");

lazy_static::lazy_static! {
    pub static ref DB_CONN: DbHandle = establish_connection();
}

/// Opens the connection to the database, which is selected by the `database_type` of the config,
/// and applies all pending migrations to it.
///
/// This is called once to fill the `DB_CONN`-singleton, which is shared by all tables.
///
/// # Returns
///
/// The handle of the open connection.
///
/// # Panics
///
/// Panics, if the config of the database is incomplete, the database can not be opened or the
/// migrations can not be applied, because the service can not work without its database.
fn establish_connection() -> DbHandle {
    let migrations = DbMigrations {
        sqlite: &SQLITE_MIGRATIONS,
        mysql: &MYSQL_MIGRATIONS,
    };

    DatabaseConfig::select(
        config::CONFIG.database_type,
        &config::CONFIG.sqlite,
        &config::CONFIG.mysql,
    )
    .and_then(|database_config| DbHandle::new(database_config, migrations))
    .unwrap_or_else(|e| panic!("{e}"))
}
