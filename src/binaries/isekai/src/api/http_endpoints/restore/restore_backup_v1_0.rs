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

use actix_web::web::{self, Json, Path};
use apistos::actix::CreatedJson;
use apistos::api_operation;
use uuid::Uuid;
use validator::Validate;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::backup_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::backup as backup_clients;

use crate::core::manifest::{MANIFEST_FILE, UnverifiedManifest};
use crate::core::processing::jobs::{Job, JobVariant};
use crate::core::processing::worker_handler::add_job_to_queue;
use crate::core::restore::RestoreJob;
use crate::core::services::Services;
use crate::database::backup_table;
use crate::database::id_map_table;
use crate::database::job_table::{self, JobEntry};

#[api_operation(
    tag = "restore",
    summary = "Restore backup",
    description = r###"Restore a backup into the project of the user.

At the moment only backups of single virtual_machines can be restored. The passphrase is checked
against the manifest of the backup, before anything is changed, so a wrong passphrase is rejected
with `400`. Afterwards the project is locked and the backup is restored by a job in the
background, which can be followed over the returned job. A project, which is already locked by
another backup or restore, is rejected with `409`. The job checks all files of the backup against
its manifest, before it changes the project, so a manipulated backup changes nothing.

Without `force` the virtual_machine is restored as new virtual_machine. It keeps its original
UUID and addresses, if they are free, otherwise it gets new ones, which are listed in the id-map
of the job. With `force` the root-disk of the existing virtual_machine with the same UUID is
replaced. A missing public-key is restored from the backup and a missing network is created from
the backup, if `include_dependencies` is set. The virtual_machine gets its power-state from the
time of the backup, or is started with `start_vms`.

If the restore fails after it has changed the project, the project stays locked, until the
restore is repeated with `force` or an admin unlocks it.

Only admins of the project can restore a backup."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn restore_backup(
    backup_uuid: Path<Uuid>,
    body: Json<BackupRestoreReq>,
    context: UserContext,
) -> Result<CreatedJson<JobResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    super::super::check_project_admin(&context)?;
    let body = body.into_inner();

    let backup = backup_table::get_backup(&backup_uuid, &context)
        .map_err(|e| map_db_uuid_get_delete_error("backup", &backup_uuid, e, Some(&context)))?;
    let state = backup
        .state()
        .map_err(|e| map_internal_error("read state of backup", e, Some(&context)))?;
    if state != BackupStatus::Ready {
        return Err(ErrorResponse::Conflict(format!(
            "Backup with UUID '{}' is in state '{state}' and can't be restored.",
            *backup_uuid
        )));
    }
    let kind = backup
        .kind()
        .map_err(|e| map_internal_error("read kind of backup", e, Some(&context)))?;
    check_restore_options(kind, &body, &context)?;

    let services = Services::connect(&context)
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    // the passphrase is checked, before the project is locked
    let content = backup_clients::read_backup_file(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &backup_uuid,
        MANIFEST_FILE,
        services.insecure(),
    )
    .await
    .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    let unverified = UnverifiedManifest::parse(&content)
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    // the key-derivation is expensive on purpose, so it doesn't block a thread of the server
    let kdf = unverified.kdf().clone();
    let passphrase = body.passphrase.clone();
    let keys = web::block(move || kdf.derive_keys(&passphrase))
        .await
        .map_err(|e| map_internal_error("derive keys of backup", e, Some(&context)))?
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    let manifest = unverified
        .verify(&keys)
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;
    if manifest.backup_uuid != *backup_uuid || manifest.kind != kind {
        return Err(ErrorResponse::BadRequest(format!(
            "The manifest doesn't belong to backup with UUID '{}'.",
            *backup_uuid
        )));
    }

    let restore = RestoreJob {
        keys,
        manifest,
        force: body.force,
        start_vms: body.start_vms,
        include_dependencies: body.include_dependencies,
    };
    let job_uuid = Uuid::new_v4();
    let project_id = context.project_id.clone();
    services
        .lock(
            &project_id,
            &job_uuid,
            &format!("restore of backup {}", *backup_uuid),
        )
        .await
        .map_err(|e| map_ainari_error_to_api_response(e, Some(&context)))?;

    if let Err(e) = job_table::add_new_job(
        &job_uuid,
        JobType::Restore,
        &backup_uuid,
        &project_id,
        &restore.options(),
        &context,
    ) {
        if let Err(unlock_error) = services.unlock(&project_id, &job_uuid).await {
            log::error!("Failed to unlock project '{project_id}' again: {unlock_error}");
        }
        return Err(map_db_write_error("add job", e, Some(&context)));
    }

    add_job_to_queue(Job {
        uuid: job_uuid,
        backup_uuid: *backup_uuid,
        project_id,
        context: context.clone(),
        variant: JobVariant::Restore(Box::new(restore)),
    });

    let entry = job_table::get_job(&job_uuid, &context)
        .map_err(|e| map_db_uuid_get_after_add_error("job", &job_uuid, e, Some(&context)))?;
    Ok(CreatedJson(to_job_resp(entry, &context)?))
}

/// Checks the options of a restore, which are supported for the backup.
///
/// # Arguments
///
/// * `kind` - Kind of the backup
/// * `body` - Options of the restore
/// * `context` - User-context of the request
///
/// # Returns
///
/// Ok, if the backup can be restored with the options, or `BadRequest`.
fn check_restore_options(
    kind: BackupKind,
    body: &BackupRestoreReq,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    if kind != BackupKind::VirtualMachine {
        return Err(ErrorResponse::BadRequest(
            "Only backups of single virtual_machines can be restored at the moment.".to_string(),
        ));
    }
    if body.prune {
        return Err(ErrorResponse::BadRequest(
            "'prune' is only supported for the restore of a whole project.".to_string(),
        ));
    }
    // the virtual_machine is reserved within the project of the token of the user
    if body
        .target_project_id
        .as_ref()
        .is_some_and(|target| *target != context.project_id)
    {
        return Err(ErrorResponse::BadRequest(
            "A backup of a virtual_machine can only be restored into the project of the token."
                .to_string(),
        ));
    }
    Ok(())
}

/// Converts a job of the database into its response, together with the UUIDs, which were
/// changed by the job.
///
/// # Arguments
///
/// * `entry` - The job
/// * `context` - User-context of the request
///
/// # Returns
///
/// The response, or an internal error, if the database contains invalid values.
fn to_job_resp(entry: JobEntry, context: &UserContext) -> Result<JobResp, ErrorResponse> {
    let job_type = entry
        .job_type()
        .map_err(|e| map_internal_error("read type of job", e, Some(context)))?;
    let state = entry
        .state()
        .map_err(|e| map_internal_error("read state of job", e, Some(context)))?;
    let id_map = id_map_table::list_id_map_of_job(&entry.uuid)
        .map_err(|e| map_db_list_error("id-map", e, Some(context)))?
        .into_iter()
        .map(|mapping| IdMapEntryResp {
            resource_type: mapping.resource_type,
            old_uuid: mapping.old_uuid,
            new_uuid: mapping.new_uuid,
        })
        .collect();

    Ok(JobResp {
        uuid: entry.uuid,
        job_type,
        backup_uuid: entry.backup_uuid,
        project_id: entry.project_id,
        state,
        step: entry.step,
        messages: entry.messages,
        id_map,
        created_at: entry.created_at,
        created_by: entry.created_by,
        started_at: entry.started_at,
        finished_at: entry.finished_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use ainari_common::secret::Secret;

    fn context() -> UserContext {
        UserContext {
            token: String::new(),
            user_id: "user".to_string(),
            project_id: "project".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        }
    }

    fn request() -> BackupRestoreReq {
        BackupRestoreReq {
            passphrase: Secret::from("passphrase"),
            force: false,
            prune: false,
            target_project_id: None,
            start_vms: false,
            include_dependencies: true,
        }
    }

    #[test]
    fn test_check_restore_options() {
        let context = context();
        assert!(check_restore_options(BackupKind::VirtualMachine, &request(), &context).is_ok());
        let same_project = BackupRestoreReq {
            target_project_id: Some("project".to_string()),
            force: true,
            ..request()
        };
        assert!(check_restore_options(BackupKind::VirtualMachine, &same_project, &context).is_ok());

        // not supported yet
        assert!(check_restore_options(BackupKind::Project, &request(), &context).is_err());
        let prune = BackupRestoreReq {
            prune: true,
            ..request()
        };
        assert!(check_restore_options(BackupKind::VirtualMachine, &prune, &context).is_err());
        let other_project = BackupRestoreReq {
            target_project_id: Some("other".to_string()),
            ..request()
        };
        assert!(
            check_restore_options(BackupKind::VirtualMachine, &other_project, &context).is_err()
        );
    }
}
