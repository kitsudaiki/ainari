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

/// Configuration structure for the application
///
/// This struct holds all configuration parameters required by the application.
/// It includes general settings, API configuration, database settings,
/// Miko endpoint configuration, and port range definitions.
#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    /// Whether debug mode is enabled
    pub debug: bool,
    /// Skip TLS certificate verification for all connections
    ///
    /// Defaults to `false` for security reasons.
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,
    // groups
    /// Configuration for API settings
    pub api: ainari_config::Api,
    /// Configuration for database connections
    pub database: ainari_config::Database,
    /// Configuration for Miko endpoint
    pub miko: ainari_config::MikoEndpoint,
    /// Port range configuration
    pub ports: Ports,
}

/// Default value for skip_tls_verification
///
/// Returns `false` to enforce TLS verification by default for security reasons.
fn default_insecure_clients() -> bool {
    false
}

/// Port range configuration
///
/// Defines the minimum and maximum ports that the application can use.
#[derive(Debug, Deserialize)]
pub struct Ports {
    /// Minimum port number
    pub min_port: u16,
    /// Maximum port number
    pub max_port: u16,
}

/// Global singleton config instance
///
/// This is a lazy-initialized global configuration that reads from
/// `/etc/ainari/torii.toml` file. The configuration is loaded only once
/// when first accessed and cached for subsequent use.
///
/// # Panics
/// This will panic if the configuration file cannot be read or parsed.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = "/etc/ainari/torii.toml";
    log::debug!("read config '{file_path}'");

    match fs::read_to_string(file_path) {
        Ok(content) => {
            log::debug!("successfully read config-file '{file_path}'");
            // Attempt to parse the TOML content into our Config struct
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

/// Global singleton for internal API key
///
/// This is a lazy-initialized global secret that reads from the
/// `INTERNAL_API_KEY` environment variable. The key is loaded only once
/// when first accessed and cached for subsequent use.
///
/// # Panics
/// This will panic if the environment variable is not set.
pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.");
        process::exit(1);
    }
});
