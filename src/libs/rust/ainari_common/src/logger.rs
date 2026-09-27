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

use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;

use env_logger::{Builder, Target};
use log::LevelFilter;

use crate::config::LogType;

/// Initializes the global logger of a service
///
/// The log-level can still be overwritten with the env-variable `RUST_LOG`.
///
/// # Arguments
///
/// * `log_type` - Target of the log-output
/// * `log_path` - Directory of the log-file, only used for `LogType::LogFile`
/// * `service_name` - Name of the service, which is used as name of the log-file
/// * `debug` - Enables debug-logs if true
///
/// # Returns
///
/// `Ok(())` if successful, otherwise the error of creating or opening the log-file
pub fn init_logger(
    log_type: LogType,
    log_path: &str,
    service_name: &str,
    debug: bool,
) -> Result<(), io::Error> {
    let mut builder = Builder::from_default_env();
    match log_type {
        LogType::Stdout => {
            builder.target(Target::Stdout);
        }
        LogType::LogFile => {
            fs::create_dir_all(log_path)?;
            let file_path = Path::new(log_path).join(format!("{service_name}.log"));
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)?;
            builder.target(Target::Pipe(Box::new(file)));
        }
    }
    builder.init();

    if !debug {
        log::set_max_level(LevelFilter::Info);
    }
    Ok(())
}
