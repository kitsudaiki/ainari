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
    /// Target of the log-output, `stdout` or `log_file`
    #[serde(default)]
    pub log_type: ainari_config::LogType,
    /// Directory of the log-file, if `log_type` is `log_file`
    #[serde(default = "ainari_config::default_log_path")]
    pub log_path: String,
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
    /// Network configuration
    #[serde(default)]
    pub network: Network,
}

/// Network configuration
///
/// Defines the addresses, which hanami hands out to the virtual machines.
#[derive(Debug, Deserialize)]
pub struct Network {
    /// Range of the floating ip-addresses in CIDR-notation, which the gateway at the edge of the
    /// network translates to the internal addresses of the virtual machines
    #[serde(default = "default_floating_ip_cidr")]
    pub floating_ip_cidr: String,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            floating_ip_cidr: default_floating_ip_cidr(),
        }
    }
}

/// Default range of the floating ip-addresses
fn default_floating_ip_cidr() -> String {
    "10.0.0.0/24".to_owned()
}

/// Default value for skip_tls_verification
///
/// This function returns the default value for the skip_tls_verification flag.
/// The default value is false, meaning TLS verification is enabled by default.
fn default_insecure_clients() -> bool {
    false
}

/// Global singleton configuration virtual_machine
///
/// This lazy static variable holds the configuration for the Hanami service.
/// It is initialized by reading from the configuration file at "/etc/ainari/hanami.toml".
/// If the file cannot be read or parsed, the program will exit with an error.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // the path of the config-file can be overwritten, which the containers of the
    // docker-compose-setup use to mount their config to another place
    let file_path = match env::var("CONFIG_FILE") {
        Ok(value) => value,
        Err(_) => "/etc/ainari/hanami.toml".to_owned(),
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
