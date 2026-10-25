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
use ainari_clients::auth::renew_token_for_project_admin;
use ainari_clients::backup as backup_clients;
use ainari_clients::project::import_project;
use ainari_common::error::AinariError;

use crate::core::manifest::{MANIFEST_FILE, UnverifiedManifest};
use crate::core::processing::jobs::{Job, JobVariant};
use crate::core::processing::worker_handler::add_job_to_queue;
use crate::core::restore::{
    PlanOptions, RestoreJob, calculate_plan, load_metadata, load_target_state,
};
use crate::core::services::Services;
use crate::database::backup_table;
use crate::database::id_map_table;
use crate::database::job_table::{self, JobEntry};

#[api_operation(
    tag = "restore",
    summary = "Restore backup",
    description = r###"Restore a backup into a project.

The passphrase is checked against the manifest of the backup and its metadata is read, before
anything is changed, so a wrong passphrase or a manipulated metadata-file is rejected with `400`.
Afterwards the plan of the restore is calculated: which resources keep their UUID, which get a
new one (`id_map`), which are overwritten and which are deleted by `prune`, and how the quota of
the project is used afterwards. With `dry_run` only the plan is returned
and nothing is changed. A restore, which would exceed the quota, is rejected with `409`.

Then the project is locked and the backup is restored by a job in the background, which can be
followed over the returned job. A project, which is already locked by another backup or restore,
is rejected with `409`. The job checks the disks and images of the backup against its manifest,
before it changes the project, so a manipulated backup changes nothing.

- Without `force` a resource keeps its UUID, if it is free. A resource, whose UUID is used in
  the project, gets a new one. Restored into another project, all resources get new UUIDs,
  because their UUIDs are still used by the project of the backup.
- With `force` every UUID is kept and existing resources are overwritten. A virtual_machine gets
  the root-disk of the backup, or is created again, if its disk-size changed.
- With `prune`, which is only allowed together with `force`, the resources of the project, which
  are not part of the backup, are deleted, so the project gets exactly the state of the backup.
- Floating ip-addresses are always allocated again. The virtual_machines get their power-state
  from the time of the backup, or are started with `start_vms`.

A backup of a project is restored into its own project, or into `target_project_id`. A project,
which doesn't exist, is created with the name and quota of the backup. Users and their
assignments to the project are never restored. A backup of a virtual_machine is restored into the
project of the token, or into `target_project_id`, which must exist. A missing public-key is
restored from the backup and a missing network is created from the backup, if
`include_dependencies` is set.

If the restore fails after it has changed the project, the project stays locked. It is unlocked
by repeating the restore of the same backup with `force`, which takes over the lock, or by an
admin.

Admins of the project can restore into their own project, all other projects, also new ones,
can only be restored by admins."###,
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
) -> Result<CreatedJson<RestoreResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    super::super::check_project_admin(&context)?;
    let body = body.into_inner();
    let map_error = |e| map_ainari_error_to_api_response(e, Some(&context));

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
    let target = restore_target(kind, &backup.project_id, &body, &context)?;
    let is_admin = super::super::is_global_admin(&context);

    let services = Services::connect(&context).await.map_err(map_error)?;

    // the passphrase is checked, before anything is changed
    let content = backup_clients::read_backup_file(
        &services.endpoints.ryokan,
        &services.token,
        services.key(),
        &backup_uuid,
        MANIFEST_FILE,
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
    if manifest.backup_uuid != *backup_uuid || manifest.kind != kind {
        return Err(ErrorResponse::BadRequest(format!(
            "The manifest doesn't belong to backup with UUID '{}'.",
            *backup_uuid
        )));
    }
    let metadata = load_metadata(&services, &backup_uuid, &manifest, &keys)
        .await
        .map_err(map_error)?;

    // plan of the restore
    let target_state = load_target_state(&services, &target, &keys)
        .await
        .map_err(map_error)?;
    if !target_state.exists {
        check_new_project(kind, is_admin, &target)?;
    }
    let options = PlanOptions {
        kind,
        force: body.force,
        prune: body.prune,
        same_project: target == manifest.project_id,
        // the quota of the backup is only applied by an admin, like the import of miko does it
        import_quota: kind == BackupKind::Project
            && is_admin
            && (!target_state.exists || body.force),
        include_dependencies: body.include_dependencies,
    };
    let plan = calculate_plan(&metadata, &target_state, &options, &target);
    if body.dry_run {
        return Ok(CreatedJson(RestoreResp {
            plan: plan.to_resp(),
            job: None,
        }));
    }
    if let Some(message) = plan.exceeded_quota() {
        return Err(ErrorResponse::Conflict(message));
    }

    // a project, which doesn't exist, is created, so it can be locked
    if plan.create_project
        && let Some(project) = &metadata.miko
    {
        import_project(
            &services.miko,
            &services.token,
            services.key(),
            &target,
            project,
            false,
            services.insecure(),
        )
        .await
        .map_err(map_error)?;
    }

    let job_uuid = Uuid::new_v4();
    lock_for_restore(&services, &target, &job_uuid, &backup_uuid, body.force)
        .await
        .map_err(map_error)?;

    let plan_resp = plan.to_resp();
    let restore = RestoreJob {
        keys,
        manifest,
        force: body.force,
        start_vms: body.start_vms,
        include_dependencies: body.include_dependencies,
        prune: body.prune,
        metadata,
        plan,
    };
    let job_context = match context_for_project(&services, &context, &target).await {
        Ok(job_context) => job_context,
        Err(e) => {
            unlock_again(&services, &target, &job_uuid).await;
            return Err(map_error(e));
        }
    };
    if let Err(e) = job_table::add_new_job(
        &job_uuid,
        JobType::Restore,
        &backup_uuid,
        &target,
        &restore.options(),
        &context,
    ) {
        unlock_again(&services, &target, &job_uuid).await;
        return Err(map_db_write_error("add job", e, Some(&context)));
    }

    add_job_to_queue(Job {
        uuid: job_uuid,
        backup_uuid: *backup_uuid,
        project_id: target,
        context: job_context,
        variant: JobVariant::Restore(Box::new(restore)),
    });

    let entry = job_table::get_job(&job_uuid, &context)
        .map_err(|e| map_db_uuid_get_after_add_error("job", &job_uuid, e, Some(&context)))?;
    Ok(CreatedJson(RestoreResp {
        plan: plan_resp,
        job: Some(to_job_resp(entry, &context)?),
    }))
}

/// Selects the project, into which a backup is restored, and checks the options of the restore.
///
/// # Arguments
///
/// * `kind` - Kind of the backup
/// * `backup_project_id` - Project of the backup
/// * `body` - Options of the restore
/// * `context` - User-context of the request
///
/// # Returns
///
/// The target-project, or `BadRequest` for invalid options and `Forbidden`, if only an admin can
/// restore into the target-project.
fn restore_target(
    kind: BackupKind,
    backup_project_id: &str,
    body: &BackupRestoreReq,
    context: &UserContext,
) -> Result<String, ErrorResponse> {
    if body.prune && kind != BackupKind::Project {
        return Err(ErrorResponse::BadRequest(
            "'prune' is only supported for the restore of a whole project.".to_string(),
        ));
    }
    // without force the project can contain resources with new UUIDs, so it would not get the
    // state of the backup
    if body.prune && !body.force {
        return Err(ErrorResponse::BadRequest(
            "'prune' is only allowed together with 'force'.".to_string(),
        ));
    }

    let target = match (&body.target_project_id, kind) {
        (Some(target), _) => target.clone(),
        (None, BackupKind::Project) => backup_project_id.to_string(),
        (None, BackupKind::VirtualMachine) => context.project_id.clone(),
    };
    if target != context.project_id && !super::super::is_global_admin(context) {
        return Err(ErrorResponse::Forbidden(format!(
            "Only admins can restore a backup into project '{target}', which is not the project of \
             the token."
        )));
    }
    Ok(target)
}

/// Checks, if a backup can create the project, into which it is restored.
fn check_new_project(kind: BackupKind, is_admin: bool, target: &str) -> Result<(), ErrorResponse> {
    if kind != BackupKind::Project {
        return Err(ErrorResponse::NotFound(format!(
            "Project '{target}' doesn't exist. A backup of a virtual_machine can only be restored \
             into an existing project."
        )));
    }
    if !is_admin {
        return Err(ErrorResponse::Forbidden(format!(
            "Project '{target}' doesn't exist and can only be created by an admin."
        )));
    }
    Ok(())
}

/// Locks the project for the restore.
///
/// A failed restore keeps its project locked. A repeated restore of the same backup with `force`
/// takes over this lock, so it can repair the project. Every other lock is a conflict.
///
/// # Arguments
///
/// * `services` - Connection to the other components
/// * `project_id` - The project
/// * `job_uuid` - The new job of the restore
/// * `backup_uuid` - The restored backup
/// * `force` - True, if the restore overwrites existing resources
///
/// # Returns
///
/// Ok, if the project is locked by the job, otherwise `Conflict`.
async fn lock_for_restore(
    services: &Services,
    project_id: &str,
    job_uuid: &Uuid,
    backup_uuid: &Uuid,
    force: bool,
) -> Result<(), AinariError> {
    let reason = format!("restore of backup {backup_uuid}");
    let conflict = match services.lock(project_id, job_uuid, &reason).await {
        Err(AinariError::Conflict(message)) if force => message,
        result => return result,
    };

    let failed = job_table::list_jobs_of_backup(backup_uuid)
        .map_err(|e| AinariError::internal(format!("Failed to list jobs: {e}")))?
        .into_iter()
        .find(|job| {
            job.project_id == project_id
                && job.job_type() == Ok(JobType::Restore)
                && job.state() == Ok(JobState::Error)
        });
    let Some(failed) = failed else {
        return Err(AinariError::Conflict(conflict));
    };
    // only the job, which holds the lock, can remove it, so this never takes over another lock
    if services.unlock(project_id, &failed.uuid).await.is_err() {
        return Err(AinariError::Conflict(conflict));
    }
    log::info!(
        "Restore-job {job_uuid} takes over the lock of the failed restore-job {} of project \
         '{project_id}'",
        failed.uuid
    );
    services.lock(project_id, job_uuid, &reason).await
}

/// Context of the user for the project, into which the backup is restored.
///
/// The other components act on the project of the token, like hanami reserves a new
/// virtual_machine within it, so a restore into another project gets a token for this project
/// over the admin-endpoint of miko.
///
/// # Returns
///
/// The context with a token for the project.
async fn context_for_project(
    services: &Services,
    context: &UserContext,
    project_id: &str,
) -> Result<UserContext, AinariError> {
    if context.project_id == project_id {
        return Ok(context.clone());
    }
    // only admins can restore into another project than the one of their token, which is checked
    // by `restore_target`, and only they get a token for another project over the admin-endpoint
    let token = renew_token_for_project_admin(
        &services.miko.public_address,
        &context.token,
        project_id,
        services.insecure(),
    )
    .await?;
    // the role within the project is only read from the token by the other components
    Ok(UserContext {
        token: token.access_token,
        project_id: project_id.to_string(),
        ..context.clone()
    })
}

/// Removes the lock of a restore again, which failed before its job was queued.
async fn unlock_again(services: &Services, project_id: &str, job_uuid: &Uuid) {
    if let Err(e) = services.unlock(project_id, job_uuid).await {
        log::error!("Failed to unlock project '{project_id}' again: {e}");
    }
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

    fn context(is_admin: bool) -> UserContext {
        UserContext {
            token: String::new(),
            user_id: "user".to_string(),
            project_id: "project".to_string(),
            is_admin: is_admin.to_string(),
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
            dry_run: false,
        }
    }

    #[test]
    fn test_restore_target() {
        let (user, admin) = (context(false), context(true));
        let project = BackupKind::Project;
        let vm = BackupKind::VirtualMachine;

        // a project-backup goes into its own project, a VM-backup into the project of the token
        assert_eq!(
            restore_target(project, "project", &request(), &user).unwrap(),
            "project"
        );
        assert!(matches!(
            restore_target(project, "backup-project", &request(), &user),
            Err(ErrorResponse::Forbidden(_))
        ));
        assert_eq!(
            restore_target(project, "backup-project", &request(), &admin).unwrap(),
            "backup-project"
        );
        assert_eq!(
            restore_target(vm, "backup-project", &request(), &user).unwrap(),
            "project"
        );

        // other projects only for admins
        let other = BackupRestoreReq {
            target_project_id: Some("other".to_string()),
            ..request()
        };
        assert!(restore_target(vm, "project", &other, &user).is_err());
        assert_eq!(
            restore_target(vm, "project", &other, &admin).unwrap(),
            "other"
        );

        // prune only with force and only for projects
        let prune = BackupRestoreReq {
            prune: true,
            ..request()
        };
        assert!(matches!(
            restore_target(project, "project", &prune, &user),
            Err(ErrorResponse::BadRequest(_))
        ));
        let prune_force = BackupRestoreReq {
            force: true,
            ..prune
        };
        assert!(restore_target(project, "project", &prune_force, &user).is_ok());
        assert!(restore_target(vm, "project", &prune_force, &user).is_err());
    }

    #[test]
    fn test_check_new_project() {
        assert!(check_new_project(BackupKind::Project, true, "new").is_ok());
        assert!(matches!(
            check_new_project(BackupKind::Project, false, "new"),
            Err(ErrorResponse::Forbidden(_))
        ));
        assert!(matches!(
            check_new_project(BackupKind::VirtualMachine, true, "new"),
            Err(ErrorResponse::NotFound(_))
        ));
    }
}
