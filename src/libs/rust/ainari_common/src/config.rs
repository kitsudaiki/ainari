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

use serde::Deserialize;

use crate::secret::Secret;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MikoEndpoint {
    pub address: String,
    #[serde(default = "default_miko_port")]
    pub port: u16,
}

fn default_miko_port() -> u16 {
    0
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoint {
    pub public_address: String,
    pub public_port: u16,
    pub internal_address: String,
    pub internal_port: u16,
}

#[derive(Debug, Deserialize, Default)]
pub struct Endpoints {
    pub hanami: Endpoint,
    pub bento: Endpoint,
    pub torii: Endpoint,
}

#[derive(Debug, Deserialize)]
pub struct Api {
    pub public_ip: String,
    pub public_port: u16,
    pub internal_ip: String,
    pub internal_port: u16,
    pub internal_api_key: Secret,
}

#[derive(Debug, Deserialize)]
pub struct Database {
    pub file_path: String,
}
