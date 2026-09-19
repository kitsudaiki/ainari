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
use std::net::Ipv4Addr;
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
    /// Network interfaces the eBPF datapath is attached to
    #[serde(default)]
    pub network: Network,
    /// Settings for local development and testing only
    #[serde(default)]
    pub development: Development,
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

/// Network interface configuration
///
/// Names the interfaces the eBPF programs are attached to at startup.
#[derive(Debug, Deserialize)]
pub struct Network {
    /// Interface of the overlay network, `overlay_ingress` is attached to it
    #[serde(default = "default_overlay_iface")]
    pub overlay_iface: String,
    /// Interface of the underlay network, `underlay_ingress` is attached to it
    #[serde(default = "default_underlay_iface")]
    pub underlay_iface: String,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            overlay_iface: default_overlay_iface(),
            underlay_iface: default_underlay_iface(),
        }
    }
}

/// Default value for overlay_iface
fn default_overlay_iface() -> String {
    "veth-gw".to_owned()
}

/// Default value for underlay_iface
fn default_underlay_iface() -> String {
    "eth0".to_owned()
}

/// Development configuration
///
/// Settings which are only meant for local development and testing and must
/// not be used in a production deployment.
#[derive(Debug, Default, Deserialize)]
pub struct Development {
    /// Run as single node: the uplink, the floating IPs and all VMs sit behind
    /// this one gateway, without underlay, tunnel or IPsec
    #[serde(default)]
    pub single_node: bool,
    /// Interface facing the outside, on which the floating IPs are served.
    /// Required if `single_node` is enabled.
    pub uplink_iface: Option<String>,
    /// Next hop behind the uplink, which all traffic leaving the virtual
    /// network is sent to. Required if `single_node` is enabled.
    pub uplink_next_hop: Option<Ipv4Addr>,
}

impl Development {
    /// Checks that the single node setup has everything it needs
    ///
    /// # Returns
    /// `Ok(())` if the section is consistent, otherwise the reason why not
    pub fn validate(&self) -> Result<(), String> {
        if !self.single_node {
            return Ok(());
        }
        if self.uplink_iface.as_deref().is_none_or(str::is_empty) {
            return Err("'development.single_node' requires 'development.uplink_iface'".to_owned());
        }
        if self.uplink_next_hop.is_none() {
            return Err(
                "'development.single_node' requires 'development.uplink_next_hop'".to_owned(),
            );
        }
        Ok(())
    }
}

/// Global singleton config virtual_machine
///
/// This is a lazy-initialized global configuration that reads from
/// `/etc/ainari/torii.toml` file. The configuration is loaded only once
/// when first accessed and cached for subsequent use.
///
/// # Panics
/// This will panic if the configuration file cannot be read or parsed.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = match env::var("CONFIG_FILE") {
        Ok(value) => value,
        Err(_) => "/etc/ainari/torii.toml".to_owned(),
    };

    log::debug!("read config '{file_path}'");

    match fs::read_to_string(file_path.clone()) {
        Ok(content) => {
            log::debug!("successfully read config-file '{file_path}'");
            // Attempt to parse the TOML content into our Config struct
            match toml::from_str(&content) {
                Ok(v) => {
                    let config: Config = v;
                    if let Err(e) = config.development.validate() {
                        log::error!("Invalid config '{file_path}': {e}");
                        process::exit(1);
                    }
                    log::info!("successfully loaded config '{file_path}'");
                    config
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

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Config {
        let path = format!(
            "{}/../../../example_configs/ainari/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn the_example_config_runs_without_single_node() {
        let config = load("torii.toml");
        assert!(!config.development.single_node);
        assert_eq!(config.network.underlay_iface, "eth0");
        assert!(config.development.validate().is_ok());
    }

    #[test]
    fn the_single_node_example_config_is_valid() {
        let config = load("torii_single_node.toml");
        assert!(config.development.single_node);
        assert_eq!(config.development.uplink_iface.as_deref(), Some("uplink0"));
        assert!(config.development.validate().is_ok());
    }

    #[test]
    fn single_node_requires_an_uplink() {
        let development = Development {
            single_node: true,
            uplink_iface: None,
            uplink_next_hop: Some(Ipv4Addr::new(10, 0, 0, 1)),
        };
        assert!(development.validate().is_err());
    }
}
