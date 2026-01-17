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
use ainari_common::secret::Secret;

/// Configuration structure for the application.
/// Contains settings for various components including debug flags,
/// storage configurations, API settings, database settings, and Miko endpoint.
///
/// # Fields
/// * `debug` - Enables debug logging when true.
/// * `skip_tls_verification` - When true, skips TLS certificate verification for insecure clients.
/// * `storage` - Configuration for storage settings.
/// * `api` - Configuration for API settings from ainari_common.
/// * `database` - Configuration for database settings from ainari_common.
/// * `miko` - Configuration for Miko endpoint settings from ainari_common.
#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    pub debug: bool,
    /// When true, skips TLS certificate verification for insecure clients.
    /// Defaults to false for security reasons.
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,
    // groups
    pub storage: Storage,
    pub api: ainari_config::Api,
    pub database: ainari_config::Database,
    pub miko: ainari_config::MikoEndpoint,
}

/// Default value for skip_tls_verification.
/// Returns false to enforce secure connections by default.
///
/// # Returns
/// bool - Always returns false.
fn default_insecure_clients() -> bool {
    false
}

/// Configuration structure for storage settings.
///
/// # Fields
/// * `tempfile_location` - Path where temporary files should be stored.
#[derive(Debug, Deserialize)]
pub struct Storage {
    pub tempfile_location: String,
}

/// Global singleton configuration instance.
/// Loads configuration from "/etc/ainari/ryokan.toml" file.
///
/// # Panics
/// Will panic and exit the program if:
/// * The configuration file cannot be read.
/// * The configuration file cannot be parsed as TOML.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = "/etc/ainari/ryokan.toml";
    log::debug!("read config '{file_path}'");

    match fs::read_to_string(file_path) {
        Ok(content) => {
            log::debug!("successfully read config-file '{file_path}'");
            match toml::from_str(&content) {
                Ok(v) => {
                    log::info!("successfully loaded config '{file_path}'");
                    v
                }
                Err(e) => {
                    log::error!("Failed to parse '{e}'");
                    log::error!("{e}");
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            log::error!("Failed read config-file '{file_path}'");
            log::error!("{e}");
            process::exit(1);
        }
    }
});

/// Global singleton for internal API key.
/// Loads the key from the "INTERNAL_API_KEY" environment variable.
///
/// # Panics
/// Will panic and exit the program if the environment variable is not set.
pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
        process::exit(1);
    }
});

/// Global singleton for Onsen registration key.
/// Loads the key from the "ONSEN_REGISTRATION_KEY" environment variable.
///
/// # Panics
/// Will panic and exit the program if the environment variable is not set.
pub static ONSEN_REGISTRATION_KEY: Lazy<Secret> =
    Lazy::new(|| match env::var("ONSEN_REGISTRATION_KEY") {
        Ok(value) => Secret::from(value),
        Err(_) => {
            log::error!("env-variable 'ONSEN_REGISTRATION_KEY' was not set.)");
            process::exit(1);
        }
    });
