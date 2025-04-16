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
use log::{info, debug, error};

#[derive(Debug, Deserialize)]
pub struct Config {
    // general values
    pub debug: bool,
    // groups
    pub storage: Storage,
    pub processing: Processing,
    pub auth: Auth,
    pub api: Api,
    pub database: Database,
}

#[derive(Debug, Deserialize)]
pub struct Storage {
    pub dataset_location: String,
    pub checkpoint_location: String,
    pub tempfile_location: String,
    pub tempfile_timeout: u32,
}

#[derive(Debug, Deserialize)]
pub struct Processing {
    pub use_of_free_memory: f32,
}

#[derive(Debug, Deserialize)]
pub struct Auth {
    pub token_key_path: String,
    pub token_expire_time: u32,
}

#[derive(Debug, Deserialize)]
pub struct Api {
    pub ip: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub host: String,
    pub port: u16,
}

// Global singleton config
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    let file_path = "/etc/openhanami/hanami.toml";
    debug!("read config '{}'", file_path);

    match fs::read_to_string(&file_path) {
        Ok(content) => {
            debug!("successfully read config-file '{}'", file_path);
            match toml::from_str(&content) {
                Ok(v) => {
                    info!("successfully loaded config '{}'", file_path);
                    return v;        
                },
                Err(e) => {

                    error!("Failed to parse '{}'", e);
                    error!("{}", e);
                    process::exit(1);
                }
            }       
        },
        Err(e) => {
            error!("Failed read config-file '{}'", file_path);
            error!("{}", e);
            process::exit(1);
        }
    }
});
