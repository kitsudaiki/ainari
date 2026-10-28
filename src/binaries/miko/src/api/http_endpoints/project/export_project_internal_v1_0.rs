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

use crate::database::project_table;
use crate::database::quota_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::exports::{MikoProjectExport, QuotaExport};
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Export project",
    description = r###"Export a project together with its quota for a backup.

Users and their assignments to the project are not part of the export, because a user can be member
of multiple projects. Users, which are not admin, can only export their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 500
)]
pub async fn export_project_internal(
    project_id: Path<String>,
    context: UserContext,
) -> Result<Json<MikoProjectExport>, ErrorResponse> {
    check_project_access(&project_id, &context)?;

    let project = project_table::get_auth_project(&project_id)
        .map_err(|e| map_db_id_get_delete_error("project", &project_id, e, Some(&context)))?;
    let quota = quota_table::get_quota(&project_id, &context)
        .map_err(|e| map_db_id_get_delete_error("quota", &project_id, e, Some(&context)))?;

    Ok(Json(MikoProjectExport {
        id: project.id,
        name: project.name,
        quota: QuotaExport {
            max_virtual_machine: quota.max_virtual_machine,
            max_image: quota.max_image,
            max_secret: quota.max_secret,
            max_network: quota.max_network,
            max_floating_ip: quota.max_floating_ip,
        },
    }))
}
