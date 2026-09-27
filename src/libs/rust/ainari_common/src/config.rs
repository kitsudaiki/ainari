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

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MikoEndpoint {
    pub address: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoint {
    pub public_address: String,
    pub internal_address: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoints {
    pub hanami: Endpoint,
    pub ryokan: Endpoint,
    pub torii: Endpoint,
    pub omamori: Endpoint,
}

#[derive(Debug, Deserialize)]
pub struct Api {
    pub public_ip: String,
    pub public_port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub file_path: String,
}

/// Target of the log-output of a service
#[derive(Debug, Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogType {
    /// Write the logs to stdout
    #[default]
    Stdout,
    /// Write the logs into a file inside of the `log_path`
    LogFile,
}

/// Default directory of the log-files
pub fn default_log_path() -> String {
    "/var/log/".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct LogConfig {
        #[serde(default)]
        log_type: LogType,
    }

    #[test]
    fn test_log_type_is_read() {
        let config: LogConfig = serde_json::from_str(r#"{"log_type": "stdout"}"#).unwrap();
        assert_eq!(config.log_type, LogType::Stdout);
        let config: LogConfig = serde_json::from_str(r#"{"log_type": "log_file"}"#).unwrap();
        assert_eq!(config.log_type, LogType::LogFile);
    }

    #[test]
    fn test_missing_log_type_defaults_to_stdout() {
        let config: LogConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config.log_type, LogType::Stdout);
    }

    #[test]
    fn test_invalid_log_type_is_rejected() {
        assert!(serde_json::from_str::<LogConfig>(r#"{"log_type": "file"}"#).is_err());
    }
}
