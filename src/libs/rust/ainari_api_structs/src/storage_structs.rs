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
use validator::Validate;

/// Path of a file or directory within the storage-location of an onsen.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct StoragePathQuery {
    /// path relative to the storage-location, an empty path is the storage-location itself
    #[serde(default)]
    #[validate(length(max = 1024))]
    pub path: String,
}

/// Entry of a directory within the storage-location.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct StorageDirectoryEntry {
    /// name of the entry within the directory
    pub name: String,
    /// true, if the entry is a directory
    pub is_dir: bool,
    /// size in bytes, 0 for directories
    pub size: u64,
}

/// Direct entries of a directory within the storage-location.
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct StorageDirectoryResp {
    /// the entries sorted by name
    pub entries: Vec<StorageDirectoryEntry>,
}

/// Size and checksum of a file within the storage-location.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, ApiComponent)]
pub struct StorageFileInfoResp {
    /// size of the file in bytes
    pub size: u64,
    /// hex-encoded sha256 of the content of the file
    pub sha256: String,
}
