// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

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
use std::fs;
use std::process;
use std::env;

use ainari_common::config as ainari_config;
use ainari_common::secret::Secret;

#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    pub debug: bool,
    #[serde(default = "default_insecure_clients")]
    pub skip_tls_verification: bool,
    pub address: String,
    // groups
    pub processing: Processing,
    pub api: ainari_config::Api,
    pub storage: Storage,
    pub database: ainari_config::Database,
    pub miko: ainari_config::MikoEndpoint,
    pub hanami: HanamiConf,
}

fn default_insecure_clients() -> bool {
    false
}

#[derive(Debug, Deserialize)]
pub struct Storage {
    pub tempfile_location: String,
}

#[derive(Debug, Deserialize)]
pub struct Processing {
    #[serde(default = "default_max_number_of_threads")]
    pub max_number_of_threads: usize,
}

fn default_max_number_of_threads() -> usize {
    0
}

#[derive(Debug, Deserialize)]
pub struct HanamiConf {
    pub registation_key: Secret,
}

// Global singleton config
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

pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| {
    match env::var("INTERNAL_API_KEY") {
        Ok(value) => Secret::from(value),
        Err(_) => {
            log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
            process::exit(1);
        }
    }
});

