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

#[derive(Debug, Deserialize, Clone)]
pub struct MikoConnection {
    pub address: String,
    #[serde(default = "default_miko_port")]
    pub port: u16,
    #[serde(default = "default_miko_insecure")]
    pub insecure: bool,
}

fn default_miko_insecure() -> bool {
    false
}

fn default_miko_port() -> u16 {
    0
}

#[derive(Debug, Deserialize, Clone)]
pub struct BentoConnection {
    pub address: String,
    #[serde(default = "default_bento_port")]
    pub port: u16,
    #[serde(default = "default_bento_insecure")]
    pub insecure: bool,
}

fn default_bento_insecure() -> bool {
    false
}

fn default_bento_port() -> u16 {
    0
}
