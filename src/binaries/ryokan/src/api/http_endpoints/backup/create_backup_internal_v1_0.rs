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
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use crate::database::backup_table::{self, BackupAddResult};
use crate::onsen_functions::select_backup_target;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "backup",
    summary = "Create backup",
    description = r###"Create a new backup, whose package is stored as one directory
`backups/<uuid>` on a backup-target.

The backup starts in the state `CREATING`, in which files can be written into the package. With
`host_uuid` the package is stored on this backup-target, otherwise ryokan selects one. Users, which
are not admin, can only create backups of their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 409,
    error_code = 500
)]
pub async fn create_backup_internal(
    body: Json<BackupCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<BackupResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    if context.is_admin != true.to_string() && body.project_id != context.project_id {
        return Err(permission_denied_response());
    }

    let host_uuid = match body.host_uuid {
        Some(host_uuid) => host_uuid,
        None => select_backup_target(&context)?.uuid,
    };

    let result =
        backup_table::add_new_backup(&body.uuid, &host_uuid, &body.project_id, None, &context)
            .map_err(|e| map_db_write_error("add backup to database", e, Some(&context)))?;
    match result {
        BackupAddResult::Added => {}
        BackupAddResult::NoBackupTarget => {
            return Err(ErrorResponse::Conflict(format!(
                "Onsen-host with UUID '{host_uuid}' doesn't exist or is no backup-target."
            )));
        }
        BackupAddResult::AlreadyExists => {
            return Err(ErrorResponse::Conflict(format!(
                "Backup with UUID '{}' already exists.",
                body.uuid
            )));
        }
    }

    let backup = backup_table::get_backup(&body.uuid, &context)
        .map_err(|e| map_db_uuid_get_after_add_error("backup", &body.uuid, e, Some(&context)))?;

    Ok(CreatedJson(super::to_backup_resp(backup, &context)?))
}
