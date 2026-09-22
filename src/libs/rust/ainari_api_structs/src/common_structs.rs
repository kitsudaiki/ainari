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

use apistos::ApiComponent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VersionResp {
    pub version: String,
    pub commit_hash: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct ReadyResp {
    pub api: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct Count {
    pub number_of_items: u64,
}

/// Serde default for every `vni` field of the API: the shared tenant.
///
/// A payload that never mentions a tenant describes exactly the setup it
/// described before tenants existed, so leaving the field out has to keep
/// working everywhere it appears.
///
/// # Arguments
/// None
///
/// # Returns
/// `VNI_DEFAULT`, i.e. 0
pub fn default_vni() -> u32 {
    torii_common::VNI_DEFAULT
}
