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
    /// Target of the log-output, `stdout` or `log_file`
    #[serde(default)]
    pub log_type: ainari_config::LogType,
    /// Directory of the log-file, if `log_type` is `log_file`
    #[serde(default = "ainari_config::default_log_path")]
    pub log_path: String,

    /// Flag to skip TLS certificate verification.
    /// When true, the application will not verify the TLS certificates of remote servers.
    /// Defaults to false for security reasons.
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,

    /// Address of the internal api of this host, which is registered at the ryokan, so the
    /// ryokan and the sakura-hosts reach it over this address.
    pub address: String,

    /// Addresses and ports of the http-server.
    pub api: ainari_config::Api,

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

/// Type of the storage-backend, which is selected by `[storage] backend`.
///
/// It is an enum, so an unknown backend already fails at the parsing of the config.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageBackendType {
    /// Files on the local disk of the onsen-host
    #[default]
    Local,
}

/// Configuration of the local storage-backend (`[storage.local]`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalStorage {
    /// Directory, where the files are stored. The volumes are stored in its subdirectory
    /// `volumes`.
    pub location: String,
    /// Address of the NFS-server, which exports the directory of the volumes to the sakura-hosts.
    /// Without it, the onsen doesn't support volumes.
    #[serde(default)]
    pub nfs_server: Option<String>,
    /// Path of the NFS-export of the directory of the volumes on the NFS-server
    #[serde(default)]
    pub nfs_export: Option<String>,
}

/// Selected storage-backend with its configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageBackend {
    /// Files on the local disk of the onsen-host
    Local(LocalStorage),
}

/// Configuration structure for storage settings.
///
/// It holds exactly one storage-backend, which is selected by `[storage] backend`. The section of
/// the selected backend must exist, otherwise the parsing of the config fails.
#[derive(Debug, Deserialize)]
#[serde(try_from = "StorageSections")]
pub struct Storage {
    /// Selected storage-backend with its configuration
    pub backend: StorageBackend,
    /// The config still uses the old `[storage] location`, which is mapped to the local backend
    pub uses_legacy_location: bool,
}

/// `[storage]`-section like it is written in the config-file, before it is checked.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StorageSections {
    #[serde(default)]
    backend: StorageBackendType,
    /// old format, which is mapped to `[storage.local] location`
    location: Option<String>,
    local: Option<LocalStorage>,
}

impl TryFrom<StorageSections> for Storage {
    type Error = String;

    fn try_from(sections: StorageSections) -> Result<Self, Self::Error> {
        let uses_legacy_location = sections.location.is_some();
        let local = match (sections.location, sections.local) {
            (Some(_), Some(_)) => {
                return Err(
                    "'storage.location' and the section [storage.local] are both set, \
                    only [storage.local] should be used"
                        .to_string(),
                );
            }
            (Some(location), None) => Some(LocalStorage {
                location,
                nfs_server: None,
                nfs_export: None,
            }),
            (None, local) => local,
        };

        let backend = match sections.backend {
            StorageBackendType::Local => {
                let local = local.ok_or(
                    "storage-backend 'local' is selected, but the section [storage.local] is missing",
                )?;
                if local.nfs_server.is_some() != local.nfs_export.is_some() {
                    return Err(
                        "'nfs_server' and 'nfs_export' of [storage.local] can only be \
                        set together"
                            .to_string(),
                    );
                }
                StorageBackend::Local(local)
            }
        };

        Ok(Storage {
            backend,
            uses_legacy_location,
        })
    }
}

/// Global singleton configuration instance.
///
/// This lazy static variable holds the application's configuration.
/// It is initialized by reading from the configuration file at `/etc/ainari/onsen.toml`.
/// If the file cannot be read or parsed, the application will exit with an error.
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // the path of the config-file can be overwritten, which the containers of the
    // docker-compose-setup use to mount their config to another place
    let file_path = match env::var("CONFIG_FILE") {
        Ok(value) => value,
        Err(_) => "/etc/ainari/onsen.toml".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Wrapper of the `[storage]`-section, so it can be parsed without the rest of the config.
    #[derive(Debug, Deserialize)]
    struct StorageOnly {
        storage: Storage,
    }

    fn parse_storage(content: &str) -> Result<Storage, toml::de::Error> {
        toml::from_str::<StorageOnly>(content).map(|config| config.storage)
    }

    #[test]
    fn test_parse_local_backend() {
        let storage = parse_storage(
            r#"
            [storage]
            backend = "local"

            [storage.local]
            location = "/var/lib/ainari/onsen"
            "#,
        )
        .unwrap();
        assert_eq!(
            storage.backend,
            StorageBackend::Local(LocalStorage {
                location: "/var/lib/ainari/onsen".to_string(),
                nfs_server: None,
                nfs_export: None,
            })
        );
        assert!(!storage.uses_legacy_location);

        // the local backend is the default
        let storage = parse_storage(
            r#"
            [storage.local]
            location = "/var/lib/ainari/onsen"
            "#,
        )
        .unwrap();
        assert!(matches!(storage.backend, StorageBackend::Local(_)));
    }

    #[test]
    fn test_parse_local_backend_with_nfs() {
        let storage = parse_storage(
            r#"
            [storage]
            backend = "local"

            [storage.local]
            location = "/var/lib/ainari/onsen"
            nfs_server = "10.0.0.5"
            nfs_export = "/var/lib/ainari/onsen/volumes"
            "#,
        )
        .unwrap();
        assert_eq!(
            storage.backend,
            StorageBackend::Local(LocalStorage {
                location: "/var/lib/ainari/onsen".to_string(),
                nfs_server: Some("10.0.0.5".to_string()),
                nfs_export: Some("/var/lib/ainari/onsen/volumes".to_string()),
            })
        );

        // only one of the nfs-values
        let err = parse_storage(
            r#"
            [storage.local]
            location = "/var/lib/ainari/onsen"
            nfs_server = "10.0.0.5"
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("nfs_export"), "{err}");

        // unknown option of the local backend
        assert!(
            parse_storage(
                r#"
                [storage.local]
                location = "/var/lib/ainari/onsen"
                nfs_path = "/export"
                "#,
            )
            .is_err()
        );
    }

    #[test]
    fn test_parse_legacy_location() {
        let storage = parse_storage(
            r#"
            [storage]
            location = "/tmp/onsen"
            "#,
        )
        .unwrap();
        assert_eq!(
            storage.backend,
            StorageBackend::Local(LocalStorage {
                location: "/tmp/onsen".to_string(),
                nfs_server: None,
                nfs_export: None,
            })
        );
        assert!(storage.uses_legacy_location);
    }

    #[test]
    fn test_parse_invalid_storage() {
        // unknown backend
        let err = parse_storage(
            r#"
            [storage]
            backend = "unknown"

            [storage.local]
            location = "/tmp/onsen"
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("unknown"), "{err}");

        // missing section of the selected backend
        let err = parse_storage(
            r#"
            [storage]
            backend = "local"
            "#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("[storage.local]"), "{err}");

        // old and new format at the same time
        assert!(
            parse_storage(
                r#"
                [storage]
                location = "/tmp/onsen"

                [storage.local]
                location = "/tmp/onsen"
                "#,
            )
            .is_err()
        );

        // unknown option
        assert!(
            parse_storage(
                r#"
                [storage]
                path = "/tmp/onsen"
                "#,
            )
            .is_err()
        );
    }

    #[test]
    fn test_parse_example_configs() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../..");
        for path in [
            "example_configs/ainari/onsen.toml",
            "testing/local_stack/configs/onsen.toml",
            "testing/local_stack/configs/onsen_backup.toml",
        ] {
            let content = fs::read_to_string(format!("{root}/{path}")).unwrap();
            let config: Config = toml::from_str(&content).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(
                matches!(config.storage.backend, StorageBackend::Local(_)),
                "{path}"
            );
            assert!(!config.storage.uses_legacy_location, "{path}");
        }
    }
}
