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

#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    pub debug: bool,
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,
    // groups
    pub storage: Storage,
    pub api: ainari_config::Api,
    pub database: ainari_config::Database,
    pub miko: ainari_config::MikoEndpoint,
}

fn default_insecure_clients() -> bool {
    false
}

#[derive(Debug, Deserialize)]
pub struct Storage {
    pub tempfile_location: String,
}

// Global singleton config
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

pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
        process::exit(1);
    }
});

pub static ONSEN_REGISTRATION_KEY: Lazy<Secret> =
    Lazy::new(|| match env::var("ONSEN_REGISTRATION_KEY") {
        Ok(value) => Secret::from(value),
        Err(_) => {
            log::error!("env-variable 'ONSEN_REGISTRATION_KEY' was not set.)");
            process::exit(1);
        }
    });
