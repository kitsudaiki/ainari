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

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;

use crate::core::backup::{BackupJob, BackupScope};

#[api_operation(
    tag = "backup",
    summary = "Create backup of project",
    description = r###"Create a backup of the whole project of the user.

The backup contains the project with its quota, the secrets and public-keys, the images and
snapshots, the networks, virtual_machines, packet-filters and floating ip-addresses and a copy of
the root-disk of every virtual_machine. Users and their assignments to the project are not part
of it.

The backup is created by a job in the background, which can be followed over the returned job.
The project is locked for the whole duration of the backup, so all resources of the project can
only be read in the meantime. A project, which is already locked by another backup or restore, is
rejected with `409`.

With `stop_vms` every running virtual_machine is shut down, while its root-disk is saved, and
started again afterwards. Otherwise the virtual_machines are only paused shortly. With
`include_images` the files of the images of the project are copied into the backup as well.

The backup is encrypted with a key, which is derived from the `passphrase`. The passphrase is not
stored, so the backup can't be restored without it."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 409,
    error_code = 500
)]
pub async fn create_project_backup(
    body: Json<ProjectBackupCreateReq>,
    context: UserContext,
) -> Result<CreatedJson<BackupJobResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;

    let body = body.into_inner();
    let backup = BackupJob {
        scope: BackupScope::Project,
        passphrase: body.passphrase,
        stop_vms: body.stop_vms,
        include_images: body.include_images,
    };
    Ok(CreatedJson(
        super::start_backup(&body.name, backup, &context).await?,
    ))
}
