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

pub mod delete_host_admin_v1_0;
pub mod get_host_admin_v1_0;
pub mod list_host_admin_v1_0;
pub mod register_host_internal_v1_0;
pub mod set_backup_target_admin_v1_0;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::host_structs::HostResp;
use ainari_api_structs::storage_structs::StorageBackendType;

use crate::database::host_table::HostEntry;

/// Returns the type of the storage-backend of a host from the database as value of the api.
///
/// # Arguments
///
/// * `host` - The host from the database
///
/// # Returns
///
/// * `Ok(StorageBackendType)` - The type of the storage-backend
/// * `Err(ErrorResponse::InternalError)` - If the database contains an unknown type
pub(crate) fn backend_type_of(host: &HostEntry) -> Result<StorageBackendType, ErrorResponse> {
    host.backend_type.parse().map_err(|e| {
        ainari_api::report_internal_error!(
            None,
            "Invalid storage-backend of host '{}' in database: {e}",
            host.uuid
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    })
}

/// Converts a host from the database into the response of the api.
///
/// # Arguments
///
/// * `host` - The host from the database
///
/// # Returns
///
/// * `Ok(HostResp)` - The response
/// * `Err(ErrorResponse::InternalError)` - If the database contains an unknown storage-backend
pub(crate) fn host_resp(host: HostEntry) -> Result<HostResp, ErrorResponse> {
    Ok(HostResp {
        backend_type: backend_type_of(&host)?,
        uuid: host.uuid,
        name: host.name,
        host_address: host.address,
        is_backup_target: host.is_backup_target,
        supports_volumes: host.supports_volumes,
        created_by: host.created_by,
        created_at: host.created_at,
        updated_by: host.updated_by,
        updated_at: host.updated_at,
    })
}
