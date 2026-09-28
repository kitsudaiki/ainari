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

//! Connection-handling of the services, which support a sqlite- and a mysql-database.
//!
//! This module is only compiled with the `mysql`-feature, because it links the client-library of
//! mysql, which is not required by the services, which only support sqlite.

use diesel::backend::Backend;
use diesel::connection::SimpleConnection;
use diesel::migration::{self, Migration, MigrationSource};
use diesel::mysql::MysqlConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use std::env;
use std::sync::{LockResult, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::config::{DatabaseConfig, MYSQL_PASSWORD_ENV};
use crate::secret::Secret;

/// Time, after which an unused mysql-connection is checked, before it is used again. The
/// mysql-server closes connections, which were idle for too long, so they have to be
/// re-established.
const MYSQL_IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(60);

/// Connection to the database of a service.
///
/// The `database_type` of the config selects the variant. All queries of the tables are written
/// against this type, so they work with both databases.
#[derive(diesel::MultiConnection)]
pub enum DbConnection {
    Sqlite(SqliteConnection),
    Mysql(MysqlConnection),
}

/// Migrations of a service for both types of databases, which are embedded into the binary of the
/// service with `embed_migrations!`.
pub struct DbMigrations {
    pub sqlite: &'static EmbeddedMigrations,
    pub mysql: &'static EmbeddedMigrations,
}

/// Borrowed migrations, because `EmbeddedMigrations` can only be applied by value, but they are
/// applied again with every re-established connection.
struct MigrationsRef(&'static EmbeddedMigrations);

impl<DB: Backend> MigrationSource<DB> for MigrationsRef {
    fn migrations(&self) -> migration::Result<Vec<Box<dyn Migration<DB>>>> {
        MigrationSource::<DB>::migrations(self.0)
    }
}

/// Handle of the connection, which is shared by all tables of a service.
///
/// Like a mutex it gives exclusive access to the connection, but it additionally re-establishes
/// a mysql-connection, which was closed by the server in the meantime.
pub struct DbHandle {
    config: DatabaseConfig,
    migrations: DbMigrations,
    conn: Mutex<DbConnection>,
    /// Point in time, when the connection was used the last time
    last_used: Mutex<Instant>,
}

impl DbHandle {
    /// Opens the connection to the database and applies all pending migrations to it.
    ///
    /// # Arguments
    ///
    /// * `config` - Config of the database, which was selected by the `database_type`
    /// * `migrations` - Migrations of the service, which create and update its tables
    ///
    /// # Returns
    ///
    /// The handle of the open connection, or an error-message, if the database can not be opened
    /// or the migrations can not be applied.
    pub fn new(config: DatabaseConfig, migrations: DbMigrations) -> Result<Self, String> {
        let conn = connect(&config, &migrations)?;
        Ok(DbHandle {
            config,
            migrations,
            conn: Mutex::new(conn),
            last_used: Mutex::new(Instant::now()),
        })
    }

    /// Locks the connection for exclusive access.
    ///
    /// A mysql-connection, which was unused for longer than `MYSQL_IDLE_CHECK_INTERVAL`, is
    /// checked before and re-established, if it was closed. If this fails, the old connection is
    /// returned, so the following query fails with an error instead of a panic, and the next call
    /// tries it again.
    ///
    /// # Returns
    ///
    /// The guard of the connection, or an error, if the mutex was poisoned.
    pub fn lock(&self) -> LockResult<MutexGuard<'_, DbConnection>> {
        let mut conn = self.conn.lock()?;

        if let DbConnection::Mysql(mysql_conn) = &mut *conn {
            let mut last_used = self
                .last_used
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            if last_used.elapsed() < MYSQL_IDLE_CHECK_INTERVAL
                || mysql_conn.batch_execute("SELECT 1").is_ok()
            {
                *last_used = Instant::now();
                return Ok(conn);
            }

            log::warn!("Connection to the mysql-database was lost, so it is re-established");
            match connect(&self.config, &self.migrations) {
                Ok(new_conn) => {
                    *conn = new_conn;
                    *last_used = Instant::now();
                }
                Err(e) => log::error!("Failed to re-establish the database-connection: {e}"),
            }
        }

        Ok(conn)
    }
}

/// Opens the connection to the database and applies all pending migrations to it.
///
/// # Arguments
///
/// * `config` - Config of the database, which was selected by the `database_type`
/// * `migrations` - Migrations of the service, which create and update its tables
///
/// # Returns
///
/// The open connection, or an error-message, if the database can not be opened or the migrations
/// can not be applied.
fn connect(config: &DatabaseConfig, migrations: &DbMigrations) -> Result<DbConnection, String> {
    match config {
        DatabaseConfig::Sqlite(sqlite_config) => {
            let mut conn = SqliteConnection::establish(&sqlite_config.file_path)
                .map_err(|e| format!("Error connecting to the sqlite-database: {e}"))?;
            conn.run_pending_migrations(MigrationsRef(migrations.sqlite))
                .map_err(|e| format!("Error applying the migrations to the database: {e}"))?;
            Ok(DbConnection::Sqlite(conn))
        }
        DatabaseConfig::Mysql(mysql_config) => {
            let password = env::var(MYSQL_PASSWORD_ENV)
                .map(Secret::from)
                .map_err(|_| {
                    format!(
                        "The env-variable '{MYSQL_PASSWORD_ENV}' with the password of the \
                         mysql-database was not set"
                    )
                })?;

            let url = mysql_config.connection_url(&password);
            let mut conn = MysqlConnection::establish(url.reveal())
                .map_err(|e| format!("Error connecting to the mysql-database: {e}"))?;
            conn.run_pending_migrations(MigrationsRef(migrations.mysql))
                .map_err(|e| format!("Error applying the migrations to the database: {e}"))?;
            Ok(DbConnection::Mysql(conn))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MysqlDatabase, SqliteDatabase};
    use diesel_migrations::embed_migrations;

    const TEST_SQLITE_MIGRATIONS: EmbeddedMigrations = embed_migrations!("test_migrations/sqlite");
    const TEST_MYSQL_MIGRATIONS: EmbeddedMigrations = embed_migrations!("test_migrations/mysql");

    /// Env-variable with the host of a mysql-server for the tests. The tests, which require a
    /// mysql-server, are skipped, if it is not set. Port, user and database are read from the
    /// env-variables `AINARI_TEST_MYSQL_PORT`, `AINARI_TEST_MYSQL_USER` and
    /// `AINARI_TEST_MYSQL_DATABASE` and the password from `MYSQL_PASSWORD_ENV`.
    const TEST_MYSQL_HOST_ENV: &str = "AINARI_TEST_MYSQL_HOST";

    fn test_migrations() -> DbMigrations {
        DbMigrations {
            sqlite: &TEST_SQLITE_MIGRATIONS,
            mysql: &TEST_MYSQL_MIGRATIONS,
        }
    }

    #[test]
    fn test_sqlite_connection_with_migrations() {
        let file_path = env::temp_dir().join(format!("ainari_test_{}.db", uuid::Uuid::new_v4()));
        let config = DatabaseConfig::Sqlite(SqliteDatabase {
            file_path: file_path.to_string_lossy().to_string(),
        });

        let handle = DbHandle::new(config, test_migrations()).expect("failed to connect");
        {
            let mut conn = handle.lock().expect("mutex poisoned");
            assert!(matches!(*conn, DbConnection::Sqlite(_)));
            // the table was created by the migration
            conn.batch_execute("INSERT INTO test_entries (id) VALUES ('entry');")
                .expect("table of the migration is missing");
        }
        drop(handle);

        // a second connection to the same database doesn't apply the migrations again
        let config = DatabaseConfig::Sqlite(SqliteDatabase {
            file_path: file_path.to_string_lossy().to_string(),
        });
        DbHandle::new(config, test_migrations()).expect("failed to connect again");

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_invalid_sqlite_path_fails() {
        let config = DatabaseConfig::Sqlite(SqliteDatabase {
            file_path: "/not/existing/dir/db".to_string(),
        });
        assert!(DbHandle::new(config, test_migrations()).is_err());
    }

    #[test]
    fn test_lost_mysql_connection_is_reestablished() {
        // only a mysql-connection can be closed by the server
        let Ok(host) = env::var(TEST_MYSQL_HOST_ENV) else {
            return;
        };
        let config = DatabaseConfig::Mysql(MysqlDatabase {
            host,
            port: env::var("AINARI_TEST_MYSQL_PORT")
                .map(|port| port.parse().expect("invalid port"))
                .unwrap_or(3306),
            user: env::var("AINARI_TEST_MYSQL_USER").expect("user not set"),
            database: env::var("AINARI_TEST_MYSQL_DATABASE").expect("database not set"),
        });
        let handle = DbHandle::new(config, test_migrations()).expect("failed to connect");

        // the server closes the connection, like after a timeout
        {
            let mut conn = handle.lock().expect("mutex poisoned");
            let _ = conn.batch_execute("KILL CONNECTION_ID()");
        }
        {
            // the connection was used just now, so it is not checked and the query fails
            let mut conn = handle.lock().expect("mutex poisoned");
            assert!(conn.batch_execute("SELECT 1").is_err());
        }

        // simulate, that the connection was unused for a long time
        *handle.last_used.lock().expect("mutex poisoned") = Instant::now()
            .checked_sub(MYSQL_IDLE_CHECK_INTERVAL)
            .expect("failed to calculate point in time");

        let mut conn = handle.lock().expect("mutex poisoned");
        conn.batch_execute("SELECT 1")
            .expect("connection was not re-established");
    }
}
