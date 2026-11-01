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

use actix_web::web::{Json, Query};
use apistos::api_operation;

use crate::database::backup_table;

use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "backup",
    summary = "List backups",
    description = r###"List all backups, which are accessible for the user. With the
query-parameter `project_id`, only the backups of this project are listed.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 500
)]
pub async fn list_backup_internal(
    query: Query<BackupListQuery>,
    context: UserContext,
) -> Result<Json<BackupListResp>, ErrorResponse> {
    let backups = backup_table::list_backups(&context, query.project_id.as_deref())
        .map_err(|e| map_db_list_error("backups", e, Some(&context)))?;

    let backups = backups
        .into_iter()
        .map(|backup| super::to_backup_resp(backup, &context))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(BackupListResp { backups }))
}
