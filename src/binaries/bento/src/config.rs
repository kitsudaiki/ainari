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

use ainari_common::config as ainari_config;

#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    pub debug: bool,
    // groups
    pub storage: Storage,
    pub api: Api,
    pub database: Database,
    pub miko: ainari_config::MikoConnection,
}

#[derive(Debug, Deserialize)]
pub struct Storage {
    pub dataset_location: String,
    pub checkpoint_location: String,
    pub tempfile_location: String,
}

#[derive(Debug, Deserialize)]
pub struct Api {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub file_path: String,
}

// Global singleton config
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = "/etc/ainari/bento.toml";
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
