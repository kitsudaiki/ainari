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

use actix_web::web::{self, Json};
use apistos::actix::CreatedJson;
use apistos::api_operation;
use validator::Validate;

use ainari_api::common_functions::*;
use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::backup::{read_unregistered_manifest, register_backup};

use crate::core::manifest::UnverifiedManifest;
use crate::core::services::Services;
use crate::database::backup_table::{self, BackupEntry};

#[api_operation(
    tag = "import",
    summary = "Import foreign backup",
    description = r###"Import a backup, which was copied onto a backup-target from outside, for example
from another deployment for a disaster-recovery.

The passphrase is checked against the manifest of the backup, before the backup is registered, so
a wrong passphrase is rejected with `400`. Afterwards the backup belongs to the project, which it
was created from, and can be restored like every other backup, also into a new project. The users
of the old deployment are not part of the backup and have to be created and assigned to the
restored project by an admin. This can only be done by an admin."###,
    error_code = 400,
    error_code = 401,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn import_backup_admin(
    body: Json<BackupImportReq>,
    context: UserContext,
) -> Result<CreatedJson<IsekaiBackupResp>, ErrorResponse> {
    check_admin_context(&context)?;
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    let body = body.into_inner();
    let map_error = |e| map_ainari_error_to_api_response(e, Some(&context));

    let known = backup_table::is_known(&body.backup_uuid)
        .map_err(|e| map_db_list_error("backups", e, Some(&context)))?;
    if known {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' already exists.",
            body.backup_uuid
        )));
    }

    let services = Services::connect(&context).await.map_err(map_error)?;

    // the passphrase is checked, before the backup is registered
    let content = read_unregistered_manifest(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &body.onsen_host_uuid,
        &body.backup_uuid,
        services.insecure(),
    )
    .await
    .map_err(map_error)?;
    let unverified = UnverifiedManifest::parse(&content).map_err(map_error)?;
    // the key-derivation is expensive on purpose, so it doesn't block a thread of the server
    let kdf = unverified.kdf().clone();
    let passphrase = body.passphrase.clone();
    let keys = web::block(move || kdf.derive_keys(&passphrase))
        .await
        .map_err(|e| map_internal_error("derive keys of backup", e, Some(&context)))?
        .map_err(map_error)?;
    let manifest = unverified.verify(&keys).map_err(map_error)?;
    if manifest.backup_uuid != body.backup_uuid {
        return Err(ErrorResponse::BadRequest(format!(
            "The manifest doesn't belong to backup with UUID '{}'.",
            body.backup_uuid
        )));
    }

    // the files of the backup are checked against the manifest by every restore
    let package = register_backup(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &body.backup_uuid,
        &manifest.project_id,
        &body.onsen_host_uuid,
        services.insecure(),
    )
    .await
    .map_err(map_error)?;

    let name = body
        .name
        .unwrap_or_else(|| format!("imported-{}", body.backup_uuid));
    backup_table::add_imported_backup(
        &body.backup_uuid,
        &name,
        manifest.kind,
        &manifest.project_id,
        manifest.virtual_machine_uuid,
        &manifest.ainari_version,
        &context,
    )
    .map_err(|e| map_db_write_error("add backup", e, Some(&context)))?;

    let entry = backup_table::get_backup(&body.backup_uuid, &context).map_err(|e| {
        map_db_uuid_get_after_add_error("backup", &body.backup_uuid, e, Some(&context))
    })?;
    Ok(CreatedJson(to_backup_resp(entry, Some(&package))?))
}

/// Converts a backup of the database into its response.
///
/// # Arguments
///
/// * `entry` - The backup
/// * `package` - The backup-resource in ryokan with the same UUID, if it is known
///
/// # Returns
///
/// The response, or an internal error, if the database contains invalid values.
fn to_backup_resp(
    entry: BackupEntry,
    package: Option<&BackupResp>,
) -> Result<IsekaiBackupResp, ErrorResponse> {
    let kind = entry
        .kind()
        .map_err(|e| map_internal_error("read kind of backup", e, None))?;
    let state = entry
        .state()
        .map_err(|e| map_internal_error("read state of backup", e, None))?;
    // the size is only known, after the package was sealed
    let sealed = package.filter(|package| package.backup_status == BackupStatus::Ready);

    Ok(IsekaiBackupResp {
        uuid: entry.uuid,
        name: entry.name,
        kind,
        project_id: entry.project_id,
        virtual_machine_uuid: entry.virtual_machine_uuid,
        state,
        ainari_version: entry.ainari_version,
        host_uuid: package.map(|package| package.host_uuid),
        file_count: sealed.map(|package| package.file_count),
        total_size: sealed.map(|package| package.total_size),
        created_at: entry.created_at,
        created_by: entry.created_by,
    })
}
