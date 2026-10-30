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
use apistos::api_operation;

use crate::database::project_table;

use ainari_api::common_functions::*;
use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::project_structs::*;
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "List projects",
    description = r###"List basic information of all projects from the database.

Every project contains its lock, while it is locked by a backup, a restore or an admin. This can
only be done by an admin."###,
    error_code = 401,
    error_code = 500
)]
pub async fn list_project_admin(
    context: UserContext,
) -> Result<Json<ProjectListResp>, ErrorResponse> {
    // validate request
    check_admin_context(&context)?;

    // get list of all projects from database
    let projects = project_table::list_projects(&context)
        .map_err(|e| map_db_list_error("projects", e, Some(&context)))?;

    let mut resp = ProjectListResp {
        projects: Vec::new(),
    };

    // convert database-output
    for project in projects {
        let lock = super::lock_of(&project, &context)?;
        let obj = ProjectBasicResp {
            id: project.id,
            name: project.name,
            lock,
        };

        resp.projects.push(obj);
    }

    Ok(Json(resp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::http_endpoints::project::import_project_internal_v1_0::import_project_internal;
    use actix_web::web::Path;
    use ainari_api_structs::exports::{MikoProjectExport, QuotaExport};
    use ainari_api_structs::imports::MikoProjectImportReq;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;
    use uuid::Uuid;

    const PROJECT_ID: &str = "test-list-project-lock";

    fn admin() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: PROJECT_ID.to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        }
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

    async fn listed_lock(context: &UserContext) -> Option<ProjectLock> {
        let resp = list_project_admin(context.clone()).await.ok().unwrap();
        resp.projects
            .iter()
            .find(|project| project.id == PROJECT_ID)
            .expect("project is not listed")
            .lock
            .clone()
    }

    #[actix_rt::test]
    #[serial]
    async fn test_list_shows_lock() {
        let project_id = PROJECT_ID.to_string();
        hard_delete(&project_id);
        let context = admin();
        let body = MikoProjectImportReq {
            project: MikoProjectExport {
                id: project_id.clone(),
                name: "lock test".to_string(),
                quota: QuotaExport {
                    max_virtual_machine: 1,
                    max_image: 1,
                    max_secret: 1,
                    max_network: 1,
                    max_floating_ip: 1,
                },
            },
            force: false,
        };
        import_project_internal(Path::from(project_id.clone()), Json(body), context.clone())
            .await
            .ok()
            .unwrap();
        assert_eq!(listed_lock(&context).await, None);

        let job_uuid = Uuid::new_v4();
        assert!(matches!(
            project_table::lock_project(&project_id, &job_uuid, "backup of the test", &context),
            Ok(project_table::LockResult::Locked)
        ));
        let lock = listed_lock(&context).await.expect("lock is not listed");
        assert_eq!(lock.locked_by_job, job_uuid);
        assert_eq!(lock.lock_reason, "backup of the test");

        assert!(matches!(
            project_table::unlock_project(&project_id, Some(&job_uuid), &context),
            Ok(project_table::UnlockResult::Unlocked)
        ));
        assert_eq!(listed_lock(&context).await, None);

        hard_delete(&project_id);
    }
}
