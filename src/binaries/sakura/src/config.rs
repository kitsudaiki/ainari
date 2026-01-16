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

/// Configuration structure for the Sakura service.
/// This struct contains all the necessary configuration parameters
/// for the service to function properly.
#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    /// Enable debug logging if set to true.
    pub debug: bool,
    /// Skip TLS verification for client connections if set to true.
    /// Defaults to false for security reasons.
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,
    /// Address where the service will be available.
    pub address: String,
    // groups
    /// Configuration for processing-related parameters.
    pub processing: Processing,
    /// Configuration for API-related parameters.
    pub api: ainari_config::Api,
    /// Configuration for storage-related parameters.
    pub storage: Storage,
    /// Configuration for database-related parameters.
    pub database: ainari_config::Database,
    /// Configuration for Miko endpoint parameters.
    pub miko: ainari_config::MikoEndpoint,
}

/// Default function for skip_tls_verification configuration.
/// Returns false to ensure TLS verification is enabled by default.
///
/// # Returns
/// bool - false by default
fn default_insecure_clients() -> bool {
    false
}

/// Configuration structure for storage parameters.
/// Contains settings related to file storage.
#[derive(Debug, Deserialize)]
pub struct Storage {
    /// Path where temporary files will be stored.
    pub tempfile_location: String,
}

/// Configuration structure for processing parameters.
/// Contains settings related to the processing pipeline.
#[derive(Debug, Deserialize)]
pub struct Processing {
    /// Maximum number of threads to use for processing.
    /// Defaults to 0 which typically means using all available cores.
    #[serde(default = "default_max_number_of_threads")]
    pub max_number_of_threads: usize,
}

/// Default function for max_number_of_threads configuration.
/// Returns 0 which typically means the system will determine the optimal number of threads.
///
/// # Returns
/// usize - 0 by default
fn default_max_number_of_threads() -> usize {
    0
}

/// Global singleton config instance.
/// This is initialized once when first accessed and remains available throughout the program's lifetime.
///
/// The configuration is loaded from a TOML file at "/etc/ainari/sakura.toml".
/// If the file cannot be read or parsed, the program will exit with an error.
///
/// # Panics
/// This will panic if the configuration file cannot be read or parsed.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = "/etc/ainari/sakura.toml";
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

/// Global singleton for the internal API key.
/// This key is used for internal API authentication and is loaded from the INTERNAL_API_KEY environment variable.
///
/// # Panics
/// This will panic if the INTERNAL_API_KEY environment variable is not set.
pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
        process::exit(1);
    }
});

/// Global singleton for the Sakura registration key.
/// This key is used for Sakura service registration and is loaded from the SAKURA_REGISTRATION_KEY environment variable.
///
/// # Panics
/// This will panic if the SAKURA_REGISTRATION_KEY environment variable is not set.
pub static SAKURA_REGISTRATION_KEY: Lazy<Secret> =
    Lazy::new(|| match env::var("SAKURA_REGISTRATION_KEY") {
        Ok(value) => Secret::from(value),
        Err(_) => {
            log::error!("env-variable 'SAKURA_REGISTRATION_KEY' was not set.)");
            process::exit(1);
        }
    });
