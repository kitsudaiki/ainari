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

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::env;
use std::fs;
use std::process;

use ainari_api_structs::mls_structs::{VerifyingKey, parse_verifying_key};
use ainari_common::config as ainari_config;
use ainari_common::secret::Secret;

/// Configuration structure for the application.
/// Contains general settings and nested configuration for various components.
///
/// # Fields
///
/// * `debug` - Enables debug mode when true.
/// * `skip_tls_verification` - When true, skips TLS certificate verification (insecure).
/// * `api` - Configuration for API endpoints and settings.
/// * `database_type` - Type of the database and so the database-group, which is used.
/// * `sqlite` - Configuration of a local sqlite-database.
/// * `mysql` - Configuration of a remote mysql-database.
/// * `miko` - Configuration for Miko endpoint.
#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    pub debug: bool,
    /// Target of the log-output, `stdout` or `log_file`
    #[serde(default)]
    pub log_type: ainari_config::LogType,
    /// Directory of the log-file, if `log_type` is `log_file`
    #[serde(default = "ainari_config::default_log_path")]
    pub log_path: String,
    /// Type of the database, which also selects the database-group, which is used
    pub database_type: ainari_config::DatabaseType,
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,
    // groups
    pub api: ainari_config::Api,
    /// Local sqlite-database, required if `database_type` is `sqlite`
    pub sqlite: Option<ainari_config::SqliteDatabase>,
    /// Remote mysql-database, required if `database_type` is `mysql`
    pub mysql: Option<ainari_config::MysqlDatabase>,
    pub miko: ainari_config::MikoEndpoint,
    /// Coordination of the MLS-groups
    pub mls: MlsConf,
}

/// Configuration of the coordination of the MLS-groups
#[derive(Debug, Deserialize)]
pub struct MlsConf {
    /// Base64-encoded Ed25519 public key of hanami, which signs the membership-grants. hanami
    /// logs it at its start.
    pub grant_public_key: String,
    /// Seconds between two key-rotations of a group, whose membership didn't change
    #[serde(default = "default_key_rotation_interval")]
    pub key_rotation_interval: u64,
}

/// Default value for key_rotation_interval: one hour
fn default_key_rotation_interval() -> u64 {
    3600
}

/// Default value for TLS verification setting.
/// Returns false to enforce secure connections by default.
///
/// # Returns
/// * `bool` - Default value for skip_tls_verification
fn default_insecure_clients() -> bool {
    false
}

/// Global singleton configuration instance.
/// Loads configuration from "/etc/ainari/izakaya.toml" file.
///
/// The configuration is loaded once at startup and shared across the application.
/// If the file cannot be read or parsed, the application will exit with an error.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // the path of the config-file can be overwritten, which the containers of the
    // docker-compose-setup use to mount their config to another place
    let file_path = match env::var("CONFIG_FILE") {
        Ok(value) => value,
        Err(_) => "/etc/ainari/izakaya.toml".to_owned(),
    };
    log::debug!("read config '{file_path}'");

    match fs::read_to_string(file_path.clone()) {
        Ok(content) => {
            log::debug!("successfully read config-file '{file_path}'");
            match toml::from_str(&content) {
                Ok(v) => {
                    log::info!("successfully loaded config '{file_path}'");
                    v
                }
                Err(e) => {
                    eprintln!("Failed to parse '{e}'");
                    eprintln!("{e}");
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed read config-file '{file_path}'");
            eprintln!("{e}");
            process::exit(1);
        }
    }
});

/// Global singleton for the public key of hanami, which signs the membership-grants.
///
/// If the configured key can't be read, the application will exit with an error.
pub static GRANT_PUBLIC_KEY: Lazy<VerifyingKey> =
    Lazy::new(|| match parse_verifying_key(&CONFIG.mls.grant_public_key) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Invalid 'mls.grant_public_key': {e}");
            process::exit(1);
        }
    });

/// Global singleton for the internal API key.
/// Loads the key from the INTERNAL_API_KEY environment variable.
///
/// If the environment variable is not set, the application will exit with an error.
pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
        process::exit(1);
    }
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_example_config_is_valid() {
        let path = format!(
            "{}/../../../example_configs/ainari/izakaya.toml",
            env!("CARGO_MANIFEST_DIR")
        );
        let config: Config = toml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(config.api.internal_port, 10423);
        assert!(parse_verifying_key(&config.mls.grant_public_key).is_ok());
        assert_eq!(config.mls.key_rotation_interval, 3600);
    }
}
