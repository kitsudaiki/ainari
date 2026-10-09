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

//! Structs of the cold migration of a virtual_machine from one sakura-host to another.
//!
//! hanami orchestrates the migration over its admin-endpoint. The sakura-hosts only provide the
//! steps: the source exports the virtual_machine (shuts it down and freezes it), the target
//! imports it (pulls its description and its files from the source and starts it) and both can
//! remove their copy of it again.

use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;

use apistos::ApiComponent;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Request of an admin to move a virtual_machine to another sakura-host
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct VirtualMachineMigrateReq {
    /// The sakura-host, which runs the virtual_machine after the migration
    pub target_host_uuid: Uuid,
}

/// Answer of hanami to an accepted migration, which runs in the background afterwards
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct VirtualMachineMigrateResp {
    pub virtual_machine_uuid: Uuid,
    pub source_host_uuid: Uuid,
    pub target_host_uuid: Uuid,
}

/// Request to the target sakura-host to take over a virtual_machine, which was exported by the
/// source sakura-host
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MigrationImportReq {
    /// The virtual_machine keeps its UUID on the new host
    pub virtual_machine_uuid: Uuid,
    /// Internal address of the source sakura-host, where the files are pulled from
    #[validate(length(min = 1))]
    pub source_address: String,
    /// Boot the virtual_machine after the import, because it was running before the migration
    pub boot: bool,
}

/// Request to the source sakura-host to unfreeze a virtual_machine, whose migration failed
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent, Validate)]
pub struct MigrationCancelReq {
    /// Boot the virtual_machine again, because it was running before the migration
    pub boot: bool,
}

/// Everything the target sakura-host needs to know about an exported virtual_machine to
/// create it again with the same identity
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema, ApiComponent)]
pub struct MigrationDescriptionResp {
    pub uuid: Uuid,
    pub name: String,
    pub number_of_cores: i32,
    /// Memory in bytes
    pub memory_size: i64,
    /// Size of the disk in GiB
    pub disk_size: i64,
    pub image_uuid: Uuid,
    pub public_key_uuid: Uuid,
    pub network_uuid: Uuid,
    pub internal_ip: Ipv4Addr,
    pub tap_name: String,
    pub mac_address: String,
    pub owner_id: String,
    pub project_id: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// Files of a virtual_machine, which are transferred during its migration
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, ApiComponent)]
#[serde(rename_all = "snake_case")]
pub enum MigrationFile {
    /// The writable root-disk
    RootDisk,
    /// The cloud-init seed-image, which is copied instead of being built again, so the target
    /// doesn't need the public-key of the virtual_machine
    Seed,
}

impl fmt::Display for MigrationFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MigrationFile::RootDisk => "root_disk",
            MigrationFile::Seed => "seed",
        };
        write!(f, "{s}")
    }
}

impl FromStr for MigrationFile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "root_disk" => Ok(MigrationFile::RootDisk),
            "seed" => Ok(MigrationFile::Seed),
            other => Err(format!(
                "Unknown migration-file '{other}', expected root_disk or seed"
            )),
        }
    }
}

/// Path of the endpoint, which streams a file of an exported virtual_machine
#[derive(Debug, Deserialize, JsonSchema, ApiComponent)]
pub struct MigrationFilePath {
    pub virtual_machine_uuid: Uuid,
    pub file: MigrationFile,
}

/// Header of the file-stream, which carries the size of the uncompressed file in bytes, so the
/// receiver can detect a truncated transfer
pub const MIGRATION_FILE_SIZE_HEADER: &str = "X-Migration-File-Size";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_file_round_trip() {
        for file in [MigrationFile::RootDisk, MigrationFile::Seed] {
            assert_eq!(file.to_string().parse::<MigrationFile>(), Ok(file));
            // the path-segment and the json-value are the same
            let json = serde_json::to_string(&file).unwrap();
            assert_eq!(json, format!("\"{file}\""));
        }
        assert!("disk".parse::<MigrationFile>().is_err());
    }
}
