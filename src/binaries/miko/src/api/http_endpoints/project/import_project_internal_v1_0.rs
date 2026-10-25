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
use validator::Validate;

use crate::database::project_table;
use crate::database::quota_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

#[api_operation(
    tag = "project",
    summary = "Import project",
    description = r###"Import a project together with its quota from a backup.

The project is imported under the ID of the path, so a backup can also be restored into another
project. A project, which doesn't exist, is created with the name and the quota of the backup,
which can only be done by an admin. An existing project is rejected with `409`, unless `force` is
set: then its name is overwritten, and its quota only, if the user is an admin, so a user can't
raise the quota of the own project with a backup. `quota_imported` shows, if the quota was applied.
Users and their assignments to the project are never changed. Users, which are not admin, can only
import into their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 404,
    error_code = 409,
    error_code = 500
)]
pub async fn import_project_internal(
    project_id: Path<String>,
    body: Json<MikoProjectImportReq>,
    context: UserContext,
) -> Result<Json<MikoProjectImportResp>, ErrorResponse> {
    body.validate()
        .map_err(|e| ErrorResponse::BadRequest(format!("Invalid input: {e}")))?;
    check_project_access(&project_id, &context)?;
    if context.is_read_only() {
        return Err(permission_denied_response());
    }
    let is_admin = context.is_admin == true.to_string();
    let project = &body.project;

    match project_table::get_auth_project(&project_id) {
        Err(enums::DbError::NotFound) => {
            if !is_admin {
                return Err(ErrorResponse::Forbidden(
                    "Only admins can create a new project from a backup.".to_string(),
                ));
            }
            // the default-project of a user is only created together with the user
            if project_table::is_default_project(&project_id) {
                return Err(ErrorResponse::BadRequest(
                    "Project-IDs starting with 'default-' are reserved.".to_string(),
                ));
            }
            project_table::add_new_project_with_quota(
                &project_id,
                &project.name,
                &project.quota,
                &context,
            )
            .map_err(|e| {
                map_db_write_error(
                    &format!("add project with ID '{project_id}' to database"),
                    e,
                    Some(&context),
                )
            })?;

            Ok(Json(MikoProjectImportResp {
                project_id: project_id.clone(),
                created: true,
                quota_imported: true,
            }))
        }
        Err(e) => Err(map_db_id_get_delete_error(
            "project",
            &project_id,
            e,
            Some(&context),
        )),
        Ok(_) if !body.force => Err(ErrorResponse::Conflict(format!(
            "Project '{project_id}' already exists. Use 'force' to overwrite it."
        ))),
        Ok(_) => {
            project_table::set_project_name(&project_id, &project.name, &context).map_err(|e| {
                map_db_id_get_delete_error("project", &project_id, e, Some(&context))
            })?;

            // the backup is only protected by the passphrase of the user, so only an admin can
            // overwrite the quota, otherwise a user could raise the quota of the own project
            if is_admin {
                let quota = &project.quota;
                quota_table::set_quota(
                    &project_id,
                    quota.max_virtual_machine,
                    quota.max_image,
                    quota.max_secret,
                    quota.max_network,
                    quota.max_floating_ip,
                    &context,
                )
                .map_err(|e| map_db_id_get_delete_error("quota", &project_id, e, Some(&context)))?;
            }

            Ok(Json(MikoProjectImportResp {
                project_id: project_id.clone(),
                created: false,
                quota_imported: is_admin,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http_endpoints::project::export_project_internal_v1_0::export_project_internal;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    const PROJECT_ID: &str = "test-import-project";

    fn context(is_admin: bool, role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: PROJECT_ID.to_string(),
            is_admin: is_admin.to_string(),
            project_role: role.to_string(),
        }
    }

    fn admin() -> UserContext {
        context(true, ProjectRole::Member)
    }

    fn exported(name: &str, limit: i32) -> MikoProjectExport {
        MikoProjectExport {
            id: "original-project".to_string(),
            name: name.to_string(),
            quota: MikoQuotaExport {
                max_virtual_machine: limit,
                max_image: limit + 1,
                max_secret: limit + 2,
                max_network: limit + 3,
                max_floating_ip: limit + 4,
            },
        }
    }

    /// Imports the project like isekai does it over the api.
    async fn import_project(
        project_id: &str,
        project: &MikoProjectExport,
        force: bool,
        context: &UserContext,
    ) -> Result<MikoProjectImportResp, ErrorResponse> {
        let body = MikoProjectImportReq {
            project: project.clone(),
            force,
        };
        import_project_internal(
            Path::from(project_id.to_string()),
            Json(body),
            context.clone(),
        )
        .await
        .map(|resp| resp.into_inner())
    }

    /// Exports the project like isekai does it over the api.
    async fn export_project(
        project_id: &str,
        context: &UserContext,
    ) -> Result<MikoProjectExport, ErrorResponse> {
        export_project_internal(Path::from(project_id.to_string()), context.clone())
            .await
            .map(|resp| resp.into_inner())
    }

    fn hard_delete(project_id: &str) {
        let mut conn = crate::database::db_handle::DB_CONN
            .lock()
            .expect("mutex poisoned");
        use diesel::prelude::*;
        {
            use crate::database::project_table::projects::dsl::*;
            let _ = diesel::delete(projects.filter(id.eq(project_id))).execute(&mut *conn);
        }
        use crate::database::quota_table::quotas::dsl::*;
        let _ = diesel::delete(quotas.filter(id.eq(project_id))).execute(&mut *conn);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_new_project() {
        let project_id = PROJECT_ID.to_string();
        hard_delete(&project_id);

        // the project is created under the ID of the import, not the one of the backup
        let resp = import_project(&project_id, &exported("restored", 20), false, &admin())
            .await
            .ok()
            .unwrap();
        assert!(resp.created);
        assert!(resp.quota_imported);

        let export = export_project(&project_id, &admin()).await.ok().unwrap();
        assert_eq!(export.id, project_id);
        assert_eq!(export.name, "restored");
        assert_eq!(export.quota, exported("restored", 20).quota);

        // without force an existing project is not touched
        assert!(matches!(
            import_project(&project_id, &exported("other name", 30), false, &admin()).await,
            Err(ErrorResponse::Conflict(_))
        ));
        assert_eq!(
            export_project(&project_id, &admin())
                .await
                .ok()
                .unwrap()
                .name,
            "restored"
        );

        hard_delete(&project_id);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_existing_project_with_force() {
        let project_id = PROJECT_ID.to_string();
        hard_delete(&project_id);
        import_project(&project_id, &exported("old name", 20), false, &admin())
            .await
            .ok()
            .unwrap();

        // an admin overwrites name and quota
        let resp = import_project(&project_id, &exported("new name", 30), true, &admin())
            .await
            .ok()
            .unwrap();
        assert!(!resp.created);
        assert!(resp.quota_imported);
        assert_eq!(
            export_project(&project_id, &admin()).await.ok().unwrap(),
            MikoProjectExport {
                id: project_id.clone(),
                ..exported("new name", 30)
            }
        );

        // a user of the project only overwrites the name, the quota stays
        let member = context(false, ProjectRole::Admin);
        let resp = import_project(&project_id, &exported("by user", 1000), true, &member)
            .await
            .ok()
            .unwrap();
        assert!(!resp.quota_imported);
        let export = export_project(&project_id, &member).await.ok().unwrap();
        assert_eq!(export.name, "by user");
        assert_eq!(export.quota, exported("new name", 30).quota);

        hard_delete(&project_id);
    }

    #[actix_rt::test]
    #[serial]
    async fn test_import_project_permissions() {
        let project_id = PROJECT_ID.to_string();
        hard_delete(&project_id);

        // only admins can create a project
        let member = context(false, ProjectRole::Admin);
        assert!(matches!(
            import_project(&project_id, &exported("new name", 20), false, &member).await,
            Err(ErrorResponse::Forbidden(_))
        ));
        assert!(matches!(
            export_project(&project_id, &member).await,
            Err(ErrorResponse::NotFound(_))
        ));

        // observers can't import anything
        import_project(&project_id, &exported("new name", 20), false, &admin())
            .await
            .ok()
            .unwrap();
        let observer = context(false, ProjectRole::Observer);
        assert!(matches!(
            import_project(&project_id, &exported("other name", 20), true, &observer).await,
            Err(ErrorResponse::Forbidden(_))
        ));

        // invalid values of the backup are rejected
        assert!(matches!(
            import_project(&project_id, &exported("x", 20), true, &admin()).await,
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(matches!(
            import_project(&project_id, &exported("other name", -1), true, &admin()).await,
            Err(ErrorResponse::BadRequest(_))
        ));

        // the default-projects are only created together with their user
        let default_project = "default-test-import-user".to_string();
        hard_delete(&default_project);
        assert!(matches!(
            import_project(
                &default_project,
                &exported("other name", 20),
                false,
                &admin()
            )
            .await,
            Err(ErrorResponse::BadRequest(_))
        ));
        assert!(project_table::get_auth_project(&default_project).is_err());

        hard_delete(&project_id);
    }
}
