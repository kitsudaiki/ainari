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

use ainari_common::config as ainari_config;
use ainari_common::kdf::KdfParams;
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
/// * `miko` - Public and internal address of miko.
/// * `processing` - Settings of the workers, which process the backup- and restore-jobs.
/// * `kdf` - Parameters of the key-derivation from the passphrases of the backups.
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
    pub miko: MikoConf,
    #[serde(default)]
    pub processing: ProcessingConf,
    #[serde(default)]
    pub kdf: KdfConf,
}

/// Default value for TLS verification setting.
/// Returns false to enforce secure connections by default.
///
/// # Returns
/// * `bool` - Default value for skip_tls_verification
fn default_insecure_clients() -> bool {
    false
}

/// Addresses of miko.
///
/// Unlike the other components, isekai also needs the internal address of miko, because the
/// project-lock and the export and import of the projects are internal endpoints of miko, which
/// are not listed in the endpoints, that miko provides.
#[derive(Debug, Deserialize)]
pub struct MikoConf {
    /// Public address, which is used for the token-validation and the endpoint-list
    pub address: String,
    /// Internal address for the internal endpoints of miko
    pub internal_address: String,
}

impl MikoConf {
    /// Public endpoint of miko, like it is used by the other components.
    ///
    /// # Returns
    ///
    /// The public endpoint of miko.
    pub fn public(&self) -> ainari_config::MikoEndpoint {
        ainari_config::MikoEndpoint {
            address: self.address.clone(),
        }
    }

    /// Endpoint of miko with its public and internal address, like it is used by the clients of
    /// the internal endpoints.
    ///
    /// # Returns
    ///
    /// The full endpoint of miko.
    pub fn endpoint(&self) -> ainari_config::Endpoint {
        ainari_config::Endpoint {
            public_address: self.address.clone(),
            internal_address: self.internal_address.clone(),
        }
    }
}

/// Settings of the processing of the jobs.
#[derive(Debug, Deserialize)]
pub struct ProcessingConf {
    /// Number of worker-threads, which process the jobs. The jobs of one project are always
    /// processed by the same worker, one after another.
    #[serde(default = "default_number_of_workers")]
    pub number_of_workers: usize,
    /// Maximum number of root-disks, which are saved at the same time on one sakura-host
    #[serde(default = "default_max_parallel_snapshots")]
    pub max_parallel_snapshots: usize,
}

impl Default for ProcessingConf {
    fn default() -> Self {
        ProcessingConf {
            number_of_workers: default_number_of_workers(),
            max_parallel_snapshots: default_max_parallel_snapshots(),
        }
    }
}

/// Default number of worker-threads
fn default_number_of_workers() -> usize {
    4
}

/// Default number of root-disks, which are saved at the same time on one sakura-host
fn default_max_parallel_snapshots() -> usize {
    2
}

/// Parameters of Argon2id, which derives the keys of a new backup from its passphrase. They are
/// stored in the manifest of each backup, so a change only affects new backups.
#[derive(Debug, Deserialize)]
pub struct KdfConf {
    /// Memory-cost in KiB
    #[serde(default = "default_kdf_memory_kib")]
    pub memory_kib: u32,
    /// Number of passes over the memory
    #[serde(default = "default_kdf_iterations")]
    pub iterations: u32,
    /// Number of lanes
    #[serde(default = "default_kdf_parallelism")]
    pub parallelism: u32,
}

impl Default for KdfConf {
    fn default() -> Self {
        KdfConf {
            memory_kib: default_kdf_memory_kib(),
            iterations: default_kdf_iterations(),
            parallelism: default_kdf_parallelism(),
        }
    }
}

impl KdfConf {
    /// Converts the config into the parameters of the key-derivation.
    ///
    /// # Returns
    ///
    /// The parameters of Argon2id.
    pub fn params(&self) -> KdfParams {
        KdfParams {
            memory_kib: self.memory_kib,
            iterations: self.iterations,
            parallelism: self.parallelism,
        }
    }
}

fn default_kdf_memory_kib() -> u32 {
    KdfParams::default().memory_kib
}

fn default_kdf_iterations() -> u32 {
    KdfParams::default().iterations
}

fn default_kdf_parallelism() -> u32 {
    KdfParams::default().parallelism
}

/// Global singleton configuration instance.
/// Loads configuration from "/etc/ainari/isekai.toml" file.
///
/// The configuration is loaded once at startup and shared across the application.
/// If the file cannot be read or parsed, the application will exit with an error.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // the path of the config-file can be overwritten, which the containers of the
    // docker-compose-setup use to mount their config to another place
    let file_path = match env::var("CONFIG_FILE") {
        Ok(value) => value,
        Err(_) => "/etc/ainari/isekai.toml".to_owned(),
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
    fn test_example_config_is_valid() {
        let content = include_str!("../../../../example_configs/ainari/isekai.toml");
        let config: Config = toml::from_str(content).unwrap();
        assert!(config.kdf.params().validate().is_ok());
        assert!(config.processing.number_of_workers > 0);
        assert!(config.processing.max_parallel_snapshots > 0);
    }

    #[test]
    fn test_defaults_of_optional_groups() {
        let content = r#"
            debug = false
            database_type = "sqlite"

            [api]
            public_ip = "0.0.0.0"
            public_port = 11425
            internal_ip = "0.0.0.0"
            internal_port = 10425

            [miko]
            address = "http://127.0.0.1:11417"
            internal_address = "http://127.0.0.1:10417"
        "#;
        let config: Config = toml::from_str(content).unwrap();
        assert_eq!(config.kdf.params(), KdfParams::default());
        assert_eq!(config.processing.number_of_workers, 4);
        assert_eq!(config.processing.max_parallel_snapshots, 2);
        assert!(!config.skip_tls_verification);
    }
}
