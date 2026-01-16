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
///
/// This struct contains various settings needed for the application to function,
/// including debug flags, TLS verification settings, and endpoint addresses.
/// The storage configuration is nested within this struct.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Flag to enable debug logging.
    pub debug: bool,

    /// Flag to skip TLS certificate verification.
    /// When true, the application will not verify the TLS certificates of remote servers.
    /// Defaults to false for security reasons.
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,

    /// Network address for the application to bind to or connect to.
    pub address: String,

    /// Configuration for storage settings.
    pub storage: Storage,

    /// Configuration for the Miko endpoint.
    pub miko: ainari_config::MikoEndpoint,
}

/// Default value for skip_tls_verification.
///
/// This function returns false, meaning TLS verification is enabled by default.
/// This is a security best practice to ensure secure communication.
fn default_insecure_clients() -> bool {
    false
}

/// Configuration structure for storage settings.
///
/// This struct contains the location where the application should store its data.
#[derive(Debug, Deserialize)]
pub struct Storage {
    /// Path to the storage location.
    /// This could be a directory path or a specific file path depending on implementation.
    pub location: String,
}

/// Global singleton configuration instance.
///
/// This lazy static variable holds the application's configuration.
/// It is initialized by reading from the configuration file at `/etc/ainari/onsen.toml`.
/// If the file cannot be read or parsed, the application will exit with an error.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = "/etc/ainari/onsen.toml";
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
///
/// This lazy static variable holds the internal API key required for authentication.
/// The key is read from the `INTERNAL_API_KEY` environment variable.
/// If the environment variable is not set, the application will exit with an error.
pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
        process::exit(1);
    }
});

/// Global singleton for the Onsen registration key.
///
/// This lazy static variable holds the registration key required for Onsen service.
/// The key is read from the `ONSEN_REGISTRATION_KEY` environment variable.
/// If the environment variable is not set, the application will exit with an error.
pub static ONSEN_REGISTRATION_KEY: Lazy<Secret> =
    Lazy::new(|| match env::var("ONSEN_REGISTRATION_KEY") {
        Ok(value) => Secret::from(value),
        Err(_) => {
            log::error!("env-variable 'ONSEN_REGISTRATION_KEY' was not set.)");
            process::exit(1);
        }
    });
