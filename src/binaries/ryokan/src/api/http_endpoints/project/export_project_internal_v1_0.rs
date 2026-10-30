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

use crate::database::image_table;

use ainari_api::common_functions::*;
use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::exports::{ImageExport, RyokanProjectExport};
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Export project",
    description = r###"Export the images and snapshots of a project for a backup.

Only the metadata is exported. The files are copied into the backup separately and their location
is not part of the export. Images, which are not `READY`, are skipped, because their file is not
complete. Backups are not part of the export. Users, which are not admin, can only export their
own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 500
)]
pub async fn export_project_internal(
    project_id: Path<String>,
    context: UserContext,
) -> Result<Json<RyokanProjectExport>, ErrorResponse> {
    check_project_access(&project_id, &context)?;

    let images = image_table::list_ready_images_of_project(&project_id)
        .map_err(|e| map_db_list_error("images", e, Some(&context)))?;

    Ok(Json(RyokanProjectExport {
        images: images
            .into_iter()
            .map(|image| ImageExport {
                uuid: image.uuid,
                name: image.name,
                file_type: image.file_type,
                file_size: image.file_size.max(0) as u64,
                is_snapshot: image.is_snapshot,
                secret_uuid: image.secret_uuid,
            })
            .collect(),
    }))
}
