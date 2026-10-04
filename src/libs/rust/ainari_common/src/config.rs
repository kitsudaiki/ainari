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

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use serde::Deserialize;

use crate::secret::Secret;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MikoEndpoint {
    pub address: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoint {
    pub public_address: String,
    pub internal_address: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoints {
    pub hanami: Endpoint,
    pub ryokan: Endpoint,
    pub torii: Endpoint,
    pub omamori: Endpoint,
    /// Server of the MLS key-packages and -messages of the gateways. It is optional, so the
    /// configs, which were written before izakaya existed, stay valid.
    #[serde(default)]
    pub izakaya: Endpoint,
}

#[derive(Debug, Deserialize)]
pub struct Api {
    pub public_ip: String,
    pub public_port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub file_path: String,
}

/// Name of the env-variable, which holds the password of the mysql-database. It is not part of
/// the config-file, so it doesn't end up in a config-map or in a file on the disk.
pub const MYSQL_PASSWORD_ENV: &str = "AINARI_MYSQL_PASSWORD";

/// Characters, which are percent-encoded within the parts of an url. Only the unreserved
/// characters of RFC 3986 are kept as they are.
const URL_PART_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Type of the database of a service. It also selects the config-group, which has to exist and
/// which is used to connect to the database.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatabaseType {
    /// Local sqlite-database-file, configured by the `[sqlite]`-group
    Sqlite,
    /// Remote mysql-database, configured by the `[mysql]`-group
    Mysql,
}

/// Config-group of a local sqlite-database
#[derive(Debug, Deserialize, Clone)]
pub struct SqliteDatabase {
    /// Path of the database-file, which is created, if it doesn't exist
    pub file_path: String,
}

/// Config-group of a mysql-database. The password is read from the env-variable
/// `MYSQL_PASSWORD_ENV`.
#[derive(Debug, Deserialize, Clone)]
pub struct MysqlDatabase {
    /// Hostname or IP-address of the database-server
    pub host: String,
    /// Port of the database-server
    #[serde(default = "default_mysql_port")]
    pub port: u16,
    /// User, which is used to log into the database-server
    pub user: String,
    /// Name of the database on the server. It has to exist already, but its tables are created
    /// by the service itself.
    pub database: String,
}

/// Config of the database, which was selected by the `database_type` of a service.
#[derive(Debug, Clone)]
pub enum DatabaseConfig {
    Sqlite(SqliteDatabase),
    Mysql(MysqlDatabase),
}

impl DatabaseConfig {
    /// Selects the config-group, which belongs to the `database_type`.
    ///
    /// # Arguments
    ///
    /// * `database_type` - Type of the database from the config
    /// * `sqlite` - Optional `[sqlite]`-group of the config
    /// * `mysql` - Optional `[mysql]`-group of the config
    ///
    /// # Returns
    ///
    /// The config of the selected database, or an error-message, if its group is missing.
    pub fn select(
        database_type: DatabaseType,
        sqlite: &Option<SqliteDatabase>,
        mysql: &Option<MysqlDatabase>,
    ) -> Result<Self, String> {
        match database_type {
            DatabaseType::Sqlite => sqlite.clone().map(DatabaseConfig::Sqlite).ok_or_else(|| {
                "The database_type is 'sqlite', but the [sqlite]-group is missing".to_string()
            }),
            DatabaseType::Mysql => mysql.clone().map(DatabaseConfig::Mysql).ok_or_else(|| {
                "The database_type is 'mysql', but the [mysql]-group is missing".to_string()
            }),
        }
    }
}

/// Default port of a mysql-server
pub fn default_mysql_port() -> u16 {
    3306
}

impl MysqlDatabase {
    /// Builds the url to connect to the database-server.
    ///
    /// User, password and database are percent-encoded, so special characters within them can
    /// not break the url.
    ///
    /// # Arguments
    ///
    /// * `password` - Password of the user
    ///
    /// # Returns
    ///
    /// The url like `mysql://user:password@host:3306/database`, which contains the password and
    /// so is a secret as well.
    pub fn connection_url(&self, password: &Secret) -> Secret {
        let encode = |value: &str| utf8_percent_encode(value, URL_PART_ENCODE_SET).to_string();
        Secret::from(format!(
            "mysql://{}:{}@{}:{}/{}",
            encode(&self.user),
            encode(password.reveal()),
            self.host,
            self.port,
            encode(&self.database)
        ))
    }
}

/// Target of the log-output of a service
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogType {
    /// Write the logs to stdout
    #[default]
    Stdout,
    /// Write the logs into a file inside of the `log_path`
    LogFile,
}

/// Default directory of the log-files
pub fn default_log_path() -> String {
    "/var/log/".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct LogConfig {
        #[serde(default)]
        log_type: LogType,
    }

    #[test]
    fn test_log_type_is_read() {
        let config: LogConfig = serde_json::from_str(r#"{"log_type": "stdout"}"#).unwrap();
        assert_eq!(config.log_type, LogType::Stdout);
        let config: LogConfig = serde_json::from_str(r#"{"log_type": "log_file"}"#).unwrap();
        assert_eq!(config.log_type, LogType::LogFile);
    }

    #[test]
    fn test_missing_log_type_defaults_to_stdout() {
        let config: LogConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.log_type, LogType::Stdout);
    }

    #[test]
    fn test_invalid_log_type_is_rejected() {
        assert!(serde_json::from_str::<LogConfig>(r#"{"log_type": "file"}"#).is_err());
    }

    #[test]
    fn test_database_type_is_read() {
        let db_type: DatabaseType = serde_json::from_str(r#""sqlite""#).unwrap();
        assert_eq!(db_type, DatabaseType::Sqlite);
        let db_type: DatabaseType = serde_json::from_str(r#""mysql""#).unwrap();
        assert_eq!(db_type, DatabaseType::Mysql);
        assert!(serde_json::from_str::<DatabaseType>(r#""postgres""#).is_err());
    }

    #[test]
    fn test_select_database_config() {
        let sqlite = Some(SqliteDatabase {
            file_path: "/tmp/db".to_string(),
        });
        let mysql = Some(MysqlDatabase {
            host: "db".to_string(),
            port: 3306,
            user: "miko".to_string(),
            database: "miko".to_string(),
        });

        let config = DatabaseConfig::select(DatabaseType::Sqlite, &sqlite, &mysql).unwrap();
        assert!(matches!(config, DatabaseConfig::Sqlite(c) if c.file_path == "/tmp/db"));
        let config = DatabaseConfig::select(DatabaseType::Mysql, &sqlite, &mysql).unwrap();
        assert!(matches!(config, DatabaseConfig::Mysql(c) if c.host == "db"));

        // the group of the selected type has to exist, the other one is optional
        assert!(DatabaseConfig::select(DatabaseType::Sqlite, &None, &mysql).is_err());
        assert!(DatabaseConfig::select(DatabaseType::Mysql, &sqlite, &None).is_err());
    }

    #[test]
    fn test_mysql_port_defaults() {
        let config: MysqlDatabase =
            serde_json::from_str(r#"{"host": "db", "user": "miko", "database": "miko"}"#).unwrap();
        assert_eq!(config.port, 3306);
    }

    #[test]
    fn test_mysql_connection_url() {
        let config = MysqlDatabase {
            host: "db.local".to_string(),
            port: 3307,
            user: "miko".to_string(),
            database: "miko_db".to_string(),
        };

        let url = config.connection_url(&Secret::from("pass"));
        assert_eq!(url.reveal(), "mysql://miko:pass@db.local:3307/miko_db");

        // special characters of the password must not break the url
        let url = config.connection_url(&Secret::from("p@ss:/#?"));
        assert_eq!(
            url.reveal(),
            "mysql://miko:p%40ss%3A%2F%23%3F@db.local:3307/miko_db"
        );
    }
}
