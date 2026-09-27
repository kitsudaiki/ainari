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
    /// Target of the log-output, `stdout` or `log_file`
    #[serde(default)]
    pub log_type: ainari_config::LogType,
    /// Directory of the log-file, if `log_type` is `log_file`
    #[serde(default = "ainari_config::default_log_path")]
    pub log_path: String,
    // groups
    pub auth: Auth,
    pub api: ainari_config::Api,
    pub database: ainari_config::Database,
    pub endpoints: ainari_config::Endpoints,
}

#[derive(Debug, Deserialize)]
pub struct Auth {
    pub token_key_path: String,
    pub token_expire_time: u64,
}

// Global singleton config
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // the path of the config-file can be overwritten, which the containers of the
    // docker-compose-setup use to mount their config to another place
    let file_path = match env::var("CONFIG_FILE") {
        Ok(value) => value,
        Err(_) => "/etc/ainari/miko.toml".to_owned(),
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

pub static INTERNAL_API_KEY: Lazy<Secret> = Lazy::new(|| match env::var("INTERNAL_API_KEY") {
    Ok(value) => Secret::from(value),
    Err(_) => {
        log::error!("env-variable 'INTERNAL_API_KEY' was not set.)");
        process::exit(1);
    }
});
