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

/// Configuration structure for the Hanami service
///
/// This struct contains all the necessary configuration parameters for the Hanami service.
/// It includes general settings, API configuration, database configuration, and Miko endpoint configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    /// Flag to enable debug mode
    pub debug: bool,
    /// Flag to skip TLS verification (insecure)
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,
    // groups
    /// API configuration
    pub api: ainari_config::Api,
    /// Database configuration
    pub database: ainari_config::Database,
    /// Miko endpoint configuration
    pub miko: ainari_config::MikoEndpoint,
}

/// Default value for skip_tls_verification
///
/// This function returns the default value for the skip_tls_verification flag.
/// The default value is false, meaning TLS verification is enabled by default.
fn default_insecure_clients() -> bool {
    false
}

/// Global singleton configuration instance
///
/// This lazy static variable holds the configuration for the Hanami service.
/// It is initialized by reading from the configuration file at "/etc/ainari/hanami.toml".
/// If the file cannot be read or parsed, the program will exit with an error.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = "/etc/ainari/hanami.toml";
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

/// Global singleton for the internal API key
///
/// This lazy static variable holds the internal API key as a Secret.
/// The key is read from the "INTERNAL_API_KEY" environment variable.
/// If the environment variable is not set, the program will exit with an error.
pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
        process::exit(1);
    }
});

/// Global singleton for the Sakura registration key
///
/// This lazy static variable holds the Sakura registration key as a Secret.
/// The key is read from the "SAKURA_REGISTRATION_KEY" environment variable.
/// If the environment variable is not set, the program will exit with an error.
pub static SAKURA_REGISTRATION_KEY: Lazy<Secret> =
    Lazy::new(|| match env::var("SAKURA_REGISTRATION_KEY") {
        Ok(value) => Secret::from(value),
        Err(_) => {
            log::error!("env-variable 'SAKURA_REGISTRATION_KEY' was not set.)");
            process::exit(1);
        }
    });
