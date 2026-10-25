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

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;
use uuid::Uuid;

use crate::database::host_table;
use crate::onsen_functions::has_backups;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::host_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "host",
    summary = "Set backup-target",
    description = r###"Set, if an onsen-host is a backup-target.

A backup-target only stores backups and is never used for normal images or snapshots. A host can
only be marked as backup-target, while no images or snapshots are stored on it, and the mark can
only be removed, while no backups are stored on it anymore. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn set_backup_target_admin(
    host_uuid: Path<Uuid>,
    body: Json<HostBackupTargetUpdateReq>,
    context: UserContext,
) -> Result<Json<HostResp>, ErrorResponse> {
    check_admin_context(&context)?;

    let host_data = host_table::get_host(&host_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("onsen-host", &host_uuid, e, Some(&context)))?;

    // backups, which were copied onto the host, but are not registered yet, are only known by
    // the onsen itself, so it is asked directly. The registered backups are checked together
    // with the update.
    if !body.is_backup_target
        && host_data.is_backup_target
        && has_backups(&host_data.address, &context).await?
    {
        return Err(ErrorResponse::Conflict(format!(
            "Onsen-host with UUID '{}' can not be unmarked as backup-target, because backups \
             are still stored on it.",
            *host_uuid
        )));
    }

    // update the mark in the database
    let changed = host_table::set_backup_target(&host_uuid, body.is_backup_target, &context)
        .map_err(|e| map_db_uuid_get_delete_error("onsen-host", &host_uuid, e, Some(&context)))?;
    if !changed && body.is_backup_target {
        return Err(ErrorResponse::Conflict(format!(
            "Onsen-host with UUID '{}' can not be marked as backup-target, because images or \
             snapshots are stored on it.",
            *host_uuid
        )));
    }
    if !changed {
        return Err(ErrorResponse::Conflict(format!(
            "Onsen-host with UUID '{}' can not be unmarked as backup-target, because backups \
             are still registered on it.",
            *host_uuid
        )));
    }

    // get updated host from database
    let host_data = host_table::get_host(&host_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("onsen-host", &host_uuid, e, Some(&context)))?;

    let resp = HostResp {
        uuid: *host_uuid,
        name: host_data.name,
        host_address: host_data.address,
        is_backup_target: host_data.is_backup_target,
        created_by: host_data.created_by,
        created_at: host_data.created_at,
        updated_by: host_data.updated_by,
        updated_at: host_data.updated_at,
    };

    Ok(Json(resp))
}
