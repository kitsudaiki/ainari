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
use uuid::Uuid;

use ainari_api_structs::mls_structs::{VerifyingKey, parse_verifying_key};
use ainari_common::config as ainari_config;
use ainari_common::secret::Secret;

/// Configuration structure for the application
///
/// This struct holds all configuration parameters required by the application.
/// It includes general settings, API configuration, database settings,
/// Miko endpoint configuration, and the configuration of the proxies.
#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    /// Whether debug mode is enabled
    pub debug: bool,
    /// Target of the log-output, `stdout` or `log_file`
    #[serde(default)]
    pub log_type: ainari_config::LogType,
    /// Directory of the log-file, if `log_type` is `log_file`
    #[serde(default = "ainari_config::default_log_path")]
    pub log_path: String,
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
    /// Proxies, which make the external api of the sakura-hosts reachable. Only the gateway at
    /// the edge of the network has them.
    #[serde(default)]
    pub proxy: Option<ProxyConf>,
    /// Network interfaces the eBPF datapath is attached to
    #[serde(default)]
    pub network: Network,
    /// Settings for local development and testing only
    #[serde(default)]
    pub development: Development,
    /// Key-exchange over the MLS-groups of the networks. Without it, the gateway doesn't take
    /// part in any group, so its encrypted routes stay without keys.
    #[serde(default)]
    pub mls: Option<MlsConf>,
}

/// Configuration of the key-exchange over the MLS-groups of the networks
#[derive(Debug, Deserialize)]
pub struct MlsConf {
    /// Base64-encoded Ed25519 public key of hanami, which signs the membership-grants. hanami
    /// logs it at its start.
    pub grant_public_key: String,
}

impl Config {
    /// Checks that the sections of the config fit together
    ///
    /// # Returns
    /// `Ok(())` if the config is consistent, otherwise the reason why not
    pub fn validate(&self) -> Result<(), String> {
        self.network.validate()?;
        if let Some(proxy) = &self.proxy {
            proxy.validate()?;
        }

        // the single node setup is the edge of the network as well, so it requires an uplink
        if self.development.single_node && self.network.uplink().is_none() {
            return Err("'development.single_node' requires 'network.uplink_iface'".to_owned());
        }

        Ok(())
    }
}

/// Default value for skip_tls_verification
///
/// Returns `false` to enforce TLS verification by default for security reasons.
fn default_insecure_clients() -> bool {
    false
}

/// Configuration of the proxies
///
/// All proxies share one listener. Torii doesn't terminate the TLS-connections, but picks the
/// proxy of a connection by the server name (SNI) of its TLS ClientHello, which is
/// `<proxy-uuid>.<domain>`. A proxy has the UUID of its virtual machine, so the user sees in the
/// address, which virtual machine it reaches. The address of a proxy stays the same, when it is
/// pointed at another sakura-host, and the clients never see the address of the sakura-host.
#[derive(Debug, Deserialize)]
pub struct ProxyConf {
    /// Domain of the proxies. It needs a wildcard-entry in the DNS, which points to this gateway.
    pub domain: String,
    /// Port of the listener of the proxies
    #[serde(default = "default_proxy_port")]
    pub port: u16,
    /// Port, under which the clients reach the listener, if it is published under another port,
    /// for example by a load-balancer. Falls back to `port`.
    #[serde(default)]
    pub public_port: Option<u16>,
    /// Address, which the listener binds to. Falls back to the public address of the api, so
    /// the api can be kept away from the outside, for example behind a tls-termination, while
    /// the proxies are still reachable.
    #[serde(default)]
    pub listen_ip: Option<String>,
}

impl ProxyConf {
    /// Checks that the domain can carry the names of the proxies
    ///
    /// # Returns
    /// `Ok(())` if the section is consistent, otherwise the reason why not
    pub fn validate(&self) -> Result<(), String> {
        let domain = self.domain();
        let valid_label = |label: &str| {
            !label.is_empty()
                && label.len() <= 63
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        };
        if domain.is_empty() || !domain.split('.').all(valid_label) {
            return Err(format!(
                "'proxy.domain' '{}' is not a valid domain",
                self.domain
            ));
        }
        Ok(())
    }

    /// Returns the domain without surrounding dots, as it is matched against the server names
    fn domain(&self) -> &str {
        self.domain.trim_matches('.')
    }

    /// Returns the address, which the listener binds to
    ///
    /// # Arguments
    /// * `api` - Api configuration, whose public address is the fallback
    ///
    /// # Returns
    /// The configured `listen_ip`, or the public address of the api, if there is none
    pub fn listen_ip<'a>(&'a self, api: &'a ainari_config::Api) -> &'a str {
        self.listen_ip.as_deref().unwrap_or(&api.public_ip)
    }

    /// Returns the address, under which the clients reach a proxy
    ///
    /// # Arguments
    /// * `proxy_uuid` - UUID of the proxy
    ///
    /// # Returns
    /// The url of the proxy, like `https://<proxy-uuid>.<domain>:<port>`, where the UUID of the
    /// proxy is the one of its virtual machine
    pub fn address(&self, proxy_uuid: &Uuid) -> String {
        let port = self.public_port.unwrap_or(self.port);
        format!("https://{proxy_uuid}.{}:{port}", self.domain())
    }

    /// Returns the proxy, which a server name belongs to
    ///
    /// # Arguments
    /// * `server_name` - Server name (SNI) of a TLS ClientHello
    ///
    /// # Returns
    /// The UUID of the proxy, or `None` if the name is not the one of a proxy of this domain
    pub fn proxy_of_server_name(&self, server_name: &str) -> Option<Uuid> {
        let server_name = server_name.trim_end_matches('.').to_ascii_lowercase();
        let label = server_name
            .strip_suffix(&self.domain().to_ascii_lowercase())?
            .strip_suffix('.')?;
        // only the hyphenated form is handed out, so no other form of the same UUID matches
        let uuid = Uuid::try_parse(label).ok()?;
        (uuid.hyphenated().to_string() == label).then_some(uuid)
    }
}

/// Default value for the port of the proxies
fn default_proxy_port() -> u16 {
    10042
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
    /// Interface facing the outside, on which the floating IPs are served. Only the gateway at
    /// the edge of the network has one. If it is set, `uplink_next_hop` is required as well.
    #[serde(default)]
    pub uplink_iface: Option<String>,
    /// Next hop behind the uplink, which all traffic leaving the virtual network is sent to
    #[serde(default)]
    pub uplink_next_hop: Option<Ipv4Addr>,
    /// Underlay-address of the gateway at the edge of the network. A gateway, which only serves
    /// virtual machines, sends everything it has no route for to that gateway. A gateway with an
    /// own uplink doesn't have one, because it is the edge itself.
    #[serde(default)]
    pub default_gateway_ip: Option<Ipv4Addr>,
    /// First kernel routing-table a tenant is given. Traffic, which leaves the eBPF-datapath
    /// towards the kernel (the IPsec-protected destinations), carries no VNI anymore, so every
    /// tenant gets a routing-table of its own, numbered `tenant_table_base + vni`. The default
    /// keeps those tables far away from the well known `local`, `main` and `default` ones.
    #[serde(default = "default_tenant_table_base")]
    pub tenant_table_base: u32,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            overlay_iface: default_overlay_iface(),
            underlay_iface: default_underlay_iface(),
            uplink_iface: None,
            uplink_next_hop: None,
            default_gateway_ip: None,
            tenant_table_base: default_tenant_table_base(),
        }
    }
}

impl Network {
    /// Checks that the uplink of the gateway has everything it needs
    ///
    /// # Returns
    /// `Ok(())` if the section is consistent, otherwise the reason why not
    pub fn validate(&self) -> Result<(), String> {
        let has_iface = !self.uplink_iface.as_deref().unwrap_or_default().is_empty();
        if has_iface && self.uplink_next_hop.is_none() {
            return Err("'network.uplink_iface' requires 'network.uplink_next_hop'".to_owned());
        }
        if !has_iface && self.uplink_next_hop.is_some() {
            return Err("'network.uplink_next_hop' requires 'network.uplink_iface'".to_owned());
        }
        if has_iface && self.default_gateway_ip.is_some() {
            return Err(
                "'network.default_gateway_ip' can not be used together with 'network.uplink_iface'"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// Returns the uplink of the gateway, if it has one
    ///
    /// # Returns
    /// The name of the uplink-interface together with its next hop, or `None` for a gateway
    /// without an uplink
    pub fn uplink(&self) -> Option<(&str, Ipv4Addr)> {
        let iface = self.uplink_iface.as_deref().filter(|it| !it.is_empty())?;
        let next_hop = self.uplink_next_hop?;

        Some((iface, next_hop))
    }

    /// Names the kernel routing-table the routes of a tenant are programmed into.
    ///
    /// The shared tenant keeps using `main`, so a setup, which never mentions a VNI, produces
    /// exactly the same kernel-state it did before tenants existed.
    ///
    /// # Arguments
    /// * `vni` - The tenant whose routing-table is wanted
    ///
    /// # Returns
    /// `None` for the shared tenant, otherwise the table-number as a string ready to be handed
    /// to `ip`
    pub fn tenant_table(&self, vni: u32) -> Option<String> {
        if vni == torii_common::VNI_DEFAULT {
            return None;
        }
        Some((self.tenant_table_base + vni).to_string())
    }
}

/// Default value for tenant_table_base
fn default_tenant_table_base() -> u32 {
    100
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
                    if let Err(e) = config.validate() {
                        eprintln!("Invalid config '{file_path}': {e}");
                        process::exit(1);
                    }
                    log::info!("successfully loaded config '{file_path}'");
                    config
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

/// Global singleton for the public key of hanami, which signs the membership-grants
///
/// `None` if the key-exchange is not configured. A configured key, which can't be read, makes the
/// application exit with an error.
pub static GRANT_PUBLIC_KEY: Lazy<Option<VerifyingKey>> = Lazy::new(|| {
    let conf = CONFIG.mls.as_ref()?;
    match parse_verifying_key(&conf.grant_public_key) {
        Ok(key) => Some(key),
        Err(e) => {
            eprintln!("Invalid 'mls.grant_public_key': {e}");
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
        assert!(parse_verifying_key(&config.mls.as_ref().unwrap().grant_public_key).is_ok());
        assert_eq!(config.network.underlay_iface, "eth0");
        assert!(config.network.uplink().is_none());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn the_single_node_example_config_is_valid() {
        let config = load("torii_single_node.toml");
        assert!(config.development.single_node);
        assert_eq!(
            config.network.uplink(),
            Some(("uplink0", Ipv4Addr::new(10, 0, 0, 1)))
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn the_public_example_config_has_an_uplink_without_single_node() {
        let config = load("torii_public.toml");
        assert!(!config.development.single_node);
        // the gateway at the edge of the network is never part of a group
        assert!(config.mls.is_none());
        assert_eq!(
            config.network.uplink(),
            Some(("veth-gw", Ipv4Addr::new(10, 0, 0, 1)))
        );
        assert!(config.proxy.is_some());
        assert!(config.validate().is_ok());
    }

    fn proxy_conf(domain: &str) -> ProxyConf {
        ProxyConf {
            domain: domain.to_owned(),
            port: 10042,
            public_port: None,
            listen_ip: None,
        }
    }

    #[test]
    fn the_address_of_a_proxy_contains_its_uuid() {
        let uuid = Uuid::new_v4();
        let mut conf = proxy_conf("ainari.localhost");
        assert_eq!(
            conf.address(&uuid),
            format!("https://{uuid}.ainari.localhost:10042")
        );

        conf.public_port = Some(443);
        assert_eq!(
            conf.address(&uuid),
            format!("https://{uuid}.ainari.localhost:443")
        );
    }

    #[test]
    fn the_server_name_names_the_proxy() {
        let uuid = Uuid::new_v4();
        let conf = proxy_conf(".Ainari.Localhost.");
        assert!(conf.validate().is_ok());

        assert_eq!(
            conf.proxy_of_server_name(&format!("{uuid}.ainari.localhost")),
            Some(uuid)
        );
        // names are case-insensitive and may be fully qualified
        let upper = format!("{uuid}.AINARI.localhost.").to_uppercase();
        assert_eq!(conf.proxy_of_server_name(&upper), Some(uuid));

        assert_eq!(conf.proxy_of_server_name("ainari.localhost"), None);
        assert_eq!(
            conf.proxy_of_server_name(&format!("{uuid}.other.localhost")),
            None
        );
        assert_eq!(
            conf.proxy_of_server_name(&format!("x.{uuid}.ainari.localhost")),
            None
        );
        assert_eq!(
            conf.proxy_of_server_name(&format!("{uuid}xainari.localhost")),
            None
        );
        let simple = uuid.simple().to_string();
        assert_eq!(
            conf.proxy_of_server_name(&format!("{simple}.ainari.localhost")),
            None
        );
    }

    #[test]
    fn the_proxy_domain_has_to_be_valid() {
        assert!(proxy_conf("").validate().is_err());
        assert!(proxy_conf("ainari..localhost").validate().is_err());
        assert!(proxy_conf("ainari_x.localhost").validate().is_err());
        assert!(proxy_conf("192-168-56-11.sslip.io").validate().is_ok());
    }

    #[test]
    fn an_uplink_requires_a_next_hop() {
        let network = Network {
            uplink_iface: Some("uplink0".to_owned()),
            uplink_next_hop: None,
            ..Default::default()
        };
        assert!(network.validate().is_err());
    }

    #[test]
    fn the_shared_tenant_uses_the_main_table() {
        let network = Network::default();
        assert_eq!(network.tenant_table(0), None);
        assert_eq!(network.tenant_table(1).as_deref(), Some("101"));
    }

    #[test]
    fn single_node_requires_an_uplink() {
        let config = Config {
            development: Development { single_node: true },
            network: Network::default(),
            ..load("torii.toml")
        };
        assert!(config.validate().is_err());
    }
}
