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

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use uuid::Uuid;

use crate::database::db_handle;
use crate::database::quota_table;

use ainari_api_structs::project_structs::{MikoQuotaExport, ProjectLock};
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the projects table
table! {
    projects (id) {
        id -> Varchar,
        name -> Varchar,
        status -> Varchar,
        locked_by_job -> Nullable<Varchar>,
        lock_reason -> Nullable<Text>,
        locked_at -> Nullable<Varchar>,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents a project entry in the database.
///
/// This struct maps to the `projects` table in the database and contains
/// all fields necessary to represent a project's state.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = projects)]
pub struct ProjectEntry {
    pub id: String,
    pub name: String,
    pub status: String,
    /// Job, which has locked the project during a backup or restore, None if it is not locked
    #[diesel(serialize_as = DbOptUuid, deserialize_as = DbOptUuid)]
    pub locked_by_job: Option<Uuid>,
    /// Description, why the project is locked
    pub lock_reason: Option<String>,
    /// Timestamp, when the project was locked
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub locked_at: Option<DateTime<Utc>>,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<String>,
}

/// Adds a new project to the database.
///
/// This function creates a new project entry with the provided parameters, together with the
/// quota of the project. It checks for admin permissions and ensures the project ID doesn't
/// already exist.
///
/// # Arguments
///
/// * `project_id` - The unique identifier for the project
/// * `project_name` - The name of the project
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(usize)` with the number of rows affected if successful
/// * An error if the operation fails (permission denied, duplicate ID, or database error)
pub fn add_new_project(
    project_id: &String,
    project_name: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    add_new_project_with_quota(project_id, project_name, &DEFAULT_QUOTA, context)
}

/// Default quota of a new project, which is created over the api.
const DEFAULT_QUOTA: MikoQuotaExport = MikoQuotaExport {
    max_virtual_machine: 10,
    max_image: 10,
    max_secret: 10,
    max_network: 10,
    max_floating_ip: 10,
};

/// Adds a new project together with the given quota to the database.
///
/// Like `add_new_project`, but the quota is given, for example from a backup, which is restored.
///
/// # Arguments
///
/// * `project_id` - The unique identifier for the project
/// * `project_name` - The name of the project
/// * `quota` - The limits of the quota of the new project
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(usize)` with the number of rows affected if successful
/// * An error if the operation fails (permission denied, duplicate ID, or database error)
pub fn add_new_project_with_quota(
    project_id: &String,
    project_name: &str,
    quota: &MikoQuotaExport,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    if context.is_admin != true.to_string() {
        return Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            Box::new("Permission denied.".to_string()),
        ));
    }

    // Check if project already exists in the database
    // Note: The same ID is allowed multiple times in the table, but only one can be active
    if get_project(project_id, context).is_ok() {
        return Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new(format!("Project with ID '{project_id}' already exist.")),
        ));
    };

    // the quota limits the resources of the project, so each project gets its own one
    quota_table::add_new_quota(
        project_id,
        quota.max_virtual_machine,
        quota.max_image,
        quota.max_secret,
        quota.max_network,
        quota.max_floating_ip,
        context,
    )?;

    let project = ProjectEntry {
        id: project_id.clone(),
        name: project_name.to_owned(),
        status: "ACTIVE".to_string(),
        locked_by_job: None,
        lock_reason: None,
        locked_at: None,
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    // delete quota again, if adding of the project failed, to avoid inconsistent database
    add_project(project).inspect_err(|_| {
        quota_table::hard_delete_quota(project_id, context);
    })
}

/// Adds a project to the database.
///
/// This is a lower-level function that performs the actual database insertion.
/// It should typically be called by `add_new_project` rather than directly.
///
/// # Arguments
///
/// * `project` - The project to add to the database
///
/// # Returns
///
/// * `Ok(usize)` with the number of rows affected if successful
/// * A database error if the operation fails
pub fn add_project(project: ProjectEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::projects::dsl::*;

    diesel::insert_into(projects)
        .values(project)
        .execute(&mut *conn)
}

/// Retrieves a project for the authentication from the database.
///
/// This function fetches a project by its ID, ensuring the project is active.
/// Unlike get_project, this function doesn't check for admin privileges.
///
/// # Arguments
///
/// * `project_id` - The ID of the project to retrieve
///
/// # Returns
///
/// * `Ok(ProjectEntry)` if the project is found
/// * `DbError::NotFound` if the project doesn't exist
/// * `DbError::InternalError` if a database error occurs
pub fn get_auth_project(project_id: &String) -> Result<ProjectEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::projects::dsl::*;
    match projects
        .filter(id.eq(project_id).and(status.eq("ACTIVE")))
        .select(ProjectEntry::as_select())
        .first::<ProjectEntry>(&mut *conn)
    {
        Ok(project) => Ok(project),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Retrieves a project from the database.
///
/// This function fetches a project by its ID, checking for admin permissions.
/// Only active projects (with status "ACTIVE") are returned.
///
/// # Arguments
///
/// * `project_id` - The ID of the project to retrieve
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(ProjectEntry)` if the project is found
/// * `DbError::NotFound` if the project doesn't exist or the user lacks permissions
/// * `DbError::InternalError` if a database error occurs
pub fn get_project(
    project_id: &String,
    context: &UserContext,
) -> Result<ProjectEntry, enums::DbError> {
    if context.is_admin != true.to_string() {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::projects::dsl::*;
    match projects
        .filter(id.eq(project_id).and(status.eq("ACTIVE")))
        .select(ProjectEntry::as_select())
        .first::<ProjectEntry>(&mut *conn)
    {
        Ok(project) => Ok(project),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all active projects in the database.
///
/// This function retrieves all projects with status "ACTIVE".
/// Non-admin users will receive an empty list.
///
/// # Arguments
///
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(Vec<ProjectEntry>)` with all active projects if successful
/// * A database error if the operation fails
pub fn list_projects(context: &UserContext) -> QueryResult<Vec<ProjectEntry>> {
    if context.is_admin != true.to_string() {
        let dummy: QueryResult<Vec<ProjectEntry>> = Ok(vec![]);
        return dummy;
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::projects::dsl::*;
    projects
        .filter(status.eq("ACTIVE"))
        .select(ProjectEntry::as_select())
        .load(&mut *conn)
}

/// Changes the name of an active project.
///
/// # Arguments
///
/// * `project_id` - The ID of the project
/// * `project_name` - The new name of the project
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(())` if the name was changed
/// * `DbError::NotFound` if the project doesn't exist
/// * `DbError::PermissionDenied` if the user is only allowed to read
/// * `DbError::InternalError` if a database error occurs
pub fn set_project_name(
    project_id: &String,
    project_name: &str,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::projects::dsl::*;
    match diesel::update(projects.filter(id.eq(project_id).and(status.eq("ACTIVE"))))
        .set((
            name.eq(project_name),
            updated_at.eq(Utc::now().to_rfc3339()),
            updated_by.eq(context.user_id.clone()),
        ))
        .execute(&mut *conn)
    {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Prefix of the default-projects, which are created together with each user.
const DEFAULT_PROJECT_PREFIX: &str = "default-";

/// Builds the ID of the default-project of a user.
///
/// # Arguments
///
/// * `user_id` - The ID of the user
///
/// # Returns
///
/// The ID of the default-project of the user
pub fn default_project_id(user_id: &str) -> String {
    format!("{DEFAULT_PROJECT_PREFIX}{user_id}")
}

/// Checks if a project-ID belongs to the reserved IDs of the default-projects.
///
/// # Arguments
///
/// * `project_id` - The ID of the project
///
/// # Returns
///
/// True, if the ID starts with the prefix of the default-projects
pub fn is_default_project(project_id: &str) -> bool {
    project_id.starts_with(DEFAULT_PROJECT_PREFIX)
}

/// Deletes a project from the database.
///
/// This function marks a project as deleted by changing its status to "DELETED".
/// It checks for admin permissions before performing the operation.
///
/// # Arguments
///
/// * `project_id` - The ID of the project to delete
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(())` if the project was successfully deleted
/// * `DbError::NotFound` if the project doesn't exist or the user lacks permissions
/// * `DbError::InternalError` if a database error occurs
pub fn delete_project(project_id: &String, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    if context.is_admin != true.to_string() {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::projects::dsl::*;
    match diesel::update(projects.filter(id.eq(project_id)))
        .set(status.eq("DELETED"))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Result of locking a project.
#[derive(Debug, PartialEq)]
pub enum LockResult {
    /// The project is locked by the job now. This is also the result, if the same job has locked
    /// it before, so the locking can be repeated.
    Locked,
    /// The project is already locked by another job
    LockedByOtherJob,
}

/// Result of unlocking a project.
#[derive(Debug, PartialEq)]
pub enum UnlockResult {
    /// The project is not locked anymore. This is also the result, if it wasn't locked before,
    /// so the unlocking can be repeated.
    Unlocked,
    /// The project is locked by another job, so the lock was kept
    LockedByOtherJob,
}

/// Returns the lock of a project, or None, if it is not locked.
///
/// # Arguments
///
/// * `project` - The project from the database
///
/// # Returns
///
/// * `Ok(Option<ProjectLock>)` - The lock of the project
/// * `Err(String)` - The database contains an incomplete lock
pub fn lock_of(project: &ProjectEntry) -> Result<Option<ProjectLock>, String> {
    match (
        &project.locked_by_job,
        &project.lock_reason,
        &project.locked_at,
    ) {
        (None, _, _) => Ok(None),
        (Some(job), Some(reason), Some(locked_at)) => Ok(Some(ProjectLock {
            locked_by_job: *job,
            lock_reason: reason.clone(),
            locked_at: *locked_at,
        })),
        _ => Err(format!(
            "Incomplete lock of project '{}' in database",
            project.id
        )),
    }
}

/// Checks, if a project is locked, for the validation of a token.
///
/// Like `get_auth_project`, this doesn't check the permissions of a user, because it is used
/// before the request reaches an endpoint.
///
/// # Arguments
///
/// * `project_id` - The ID of the project
///
/// # Returns
///
/// * `Ok(bool)` - True, if the project is locked
/// * `DbError::NotFound` if the project doesn't exist
/// * `DbError::InternalError` if a database error occurs
pub fn is_project_locked(project_id: &String) -> Result<bool, enums::DbError> {
    Ok(get_auth_project(project_id)?.locked_by_job.is_some())
}

/// Locks a project for a job, like a backup or a restore.
///
/// The check and the lock are done by one conditional update, so of two jobs, which lock the same
/// project at the same time, only one gets the lock.
///
/// # Arguments
///
/// * `project_id` - The ID of the project to lock
/// * `job_uuid` - The job, which locks the project
/// * `reason` - Description, why the project is locked
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(LockResult)` - If the project is locked by the job now
/// * `DbError::NotFound` if the project doesn't exist
/// * `DbError::PermissionDenied` if the user is only allowed to read
/// * `DbError::InternalError` if a database error occurs
pub fn lock_project(
    project_id: &String,
    job_uuid: &Uuid,
    reason: &str,
    context: &UserContext,
) -> Result<LockResult, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let result = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::projects::dsl::*;
        diesel::update(
            projects.filter(
                id.eq(project_id).and(status.eq("ACTIVE")).and(
                    locked_by_job
                        .is_null()
                        .or(locked_by_job.eq(job_uuid.to_string())),
                ),
            ),
        )
        .set((
            locked_by_job.eq(Some(job_uuid.to_string())),
            lock_reason.eq(Some(reason)),
            locked_at.eq(Some(Utc::now().to_rfc3339())),
            updated_at.eq(Utc::now().to_rfc3339()),
            updated_by.eq(context.user_id.clone()),
        ))
        .execute(&mut *conn)
    };

    match result {
        Ok(0) => {
            // the project doesn't exist or is locked by another job
            get_auth_project(project_id)?;
            Ok(LockResult::LockedByOtherJob)
        }
        Ok(_) => Ok(LockResult::Locked),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Removes the lock of a project.
///
/// With `job_uuid` only the lock of this job is removed. Without it, every lock is removed,
/// which is only done by an admin, for example after a failed restore.
///
/// # Arguments
///
/// * `project_id` - The ID of the project to unlock
/// * `job_uuid` - The job, which has locked the project, or None to remove any lock
/// * `context` - The user context containing authentication information
///
/// # Returns
///
/// * `Ok(UnlockResult)` - If the project is unlocked now
/// * `DbError::NotFound` if the project doesn't exist
/// * `DbError::PermissionDenied` if the user is only allowed to read
/// * `DbError::InternalError` if a database error occurs
pub fn unlock_project(
    project_id: &String,
    job_uuid: Option<&Uuid>,
    context: &UserContext,
) -> Result<UnlockResult, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let result = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::projects::dsl::*;
        let project = projects.filter(id.eq(project_id).and(status.eq("ACTIVE")));
        let unlocked = (
            locked_by_job.eq(None::<String>),
            lock_reason.eq(None::<String>),
            locked_at.eq(None::<String>),
            updated_at.eq(Utc::now().to_rfc3339()),
            updated_by.eq(context.user_id.clone()),
        );
        match job_uuid {
            Some(job_uuid) => {
                diesel::update(project.filter(locked_by_job.eq(job_uuid.to_string())))
                    .set(unlocked)
                    .execute(&mut *conn)
            }
            None => diesel::update(project).set(unlocked).execute(&mut *conn),
        }
    };

    match result {
        Ok(0) => {
            // the project doesn't exist, isn't locked or is locked by another job
            let project = get_auth_project(project_id)?;
            match project.locked_by_job {
                None => Ok(UnlockResult::Unlocked),
                Some(_) => Ok(UnlockResult::LockedByOtherJob),
            }
        }
        Ok(_) => Ok(UnlockResult::Unlocked),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ainari_common::enums::ProjectRole;
    use serial_test::serial;

    fn hard_delete_project(project_id: &String) {
        use self::projects::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(projects.filter(id.eq(project_id))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_project() {
        let project_id = "test-project-1".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let project = ProjectEntry {
            id: project_id.clone(),
            name: "Alice".to_string(),
            status: "ACTIVE".to_string(),
            locked_by_job: None,
            lock_reason: None,
            locked_at: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_project(&project.id);

        add_project(project.clone()).unwrap();
        if let Ok(retrieved_project) = get_project(&project_id, &context) {
            assert_eq!(retrieved_project.id, project.id);
            assert_eq!(retrieved_project.name, project.name);
            assert_eq!(retrieved_project.status, project.status);
            assert_eq!(retrieved_project.created_by, project.created_by);
            assert_eq!(retrieved_project.updated_by, project.updated_by);
            assert_eq!(retrieved_project.deleted_at, project.deleted_at);
            assert_eq!(retrieved_project.deleted_by, project.deleted_by);
        };

        let _ = delete_project(&project.id, &context);
    }

    #[test]
    #[serial]
    fn test_list_projects() {
        let project_id1 = "test-project-2".to_string();
        let project_id2 = "test-project-3".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id1.clone(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let project1 = ProjectEntry {
            id: project_id1.clone(),
            name: "Alice".to_string(),
            status: "ACTIVE".to_string(),
            locked_by_job: None,
            lock_reason: None,
            locked_at: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let project2 = ProjectEntry {
            id: project_id2.clone(),
            name: "Bob".to_string(),
            status: "DELETED".to_string(),
            locked_by_job: None,
            lock_reason: None,
            locked_at: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_project(&project1.id);
        hard_delete_project(&project2.id);

        add_project(project1.clone()).unwrap();
        add_project(project2.clone()).unwrap();

        let projects = list_projects(&context).unwrap();
        assert_eq!(projects.len(), 1);

        let _ = delete_project(&project1.id, &context);
        let _ = delete_project(&project2.id, &context);
    }

    #[test]
    #[serial]
    fn test_delete_project() {
        let project_id = "test-project-5".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let project = ProjectEntry {
            id: project_id.clone(),
            name: "Alice".to_string(),
            status: "ACTIVE".to_string(),
            locked_by_job: None,
            lock_reason: None,
            locked_at: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_project(&project.id);

        add_project(project).unwrap();
        let _ = delete_project(&project_id, &context);
        let result = get_project(&project_id, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_add_new_project_creates_quota() {
        let project_id = "test-project-quota".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: "admin".to_string(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        hard_delete_project(&project_id);
        quota_table::hard_delete_quota(&project_id, &context);

        add_new_project(&project_id, "Quota", &context).unwrap();
        let Ok(quota) = quota_table::get_quota(&project_id, &context) else {
            panic!("quota of the new project was not created");
        };
        assert_eq!(quota.id, project_id);
        assert_eq!(quota.max_secret, 10);

        // a second project with the same ID is rejected and doesn't touch the existing quota
        assert!(add_new_project(&project_id, "Quota", &context).is_err());
        assert!(quota_table::get_quota(&project_id, &context).is_ok());

        hard_delete_project(&project_id);
        quota_table::hard_delete_quota(&project_id, &context);
    }

    fn lock_context(role: ProjectRole) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-lock-project".to_string(),
            is_admin: false.to_string(),
            project_role: role.to_string(),
        }
    }

    fn add_lock_test_project() -> String {
        let project_id = "test-lock-project".to_string();
        hard_delete_project(&project_id);
        add_project(ProjectEntry {
            id: project_id.clone(),
            name: "lock".to_string(),
            status: "ACTIVE".to_string(),
            locked_by_job: None,
            lock_reason: None,
            locked_at: None,
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        })
        .unwrap();
        project_id
    }

    fn current_lock(project_id: &String) -> Option<ProjectLock> {
        match get_auth_project(project_id) {
            Ok(project) => lock_of(&project).unwrap(),
            Err(_) => panic!("project not found"),
        }
    }

    #[test]
    #[serial]
    fn test_lock_project() {
        let project_id = add_lock_test_project();
        let context = lock_context(ProjectRole::Member);
        let job = Uuid::new_v4();
        let other_job = Uuid::new_v4();

        assert!(matches!(is_project_locked(&project_id), Ok(false)));
        assert_eq!(current_lock(&project_id), None);

        // lock the project
        assert!(matches!(
            lock_project(&project_id, &job, "backup", &context),
            Ok(LockResult::Locked)
        ));
        assert!(matches!(is_project_locked(&project_id), Ok(true)));
        let lock = current_lock(&project_id).unwrap();
        assert_eq!(lock.locked_by_job, job);
        assert_eq!(lock.lock_reason, "backup");

        // the same job can lock it again, another job not
        assert!(matches!(
            lock_project(&project_id, &job, "backup", &context),
            Ok(LockResult::Locked)
        ));
        assert!(matches!(
            lock_project(&project_id, &other_job, "restore", &context),
            Ok(LockResult::LockedByOtherJob)
        ));
        assert_eq!(current_lock(&project_id).unwrap().locked_by_job, job);

        // only the job, which has locked the project, can unlock it
        assert!(matches!(
            unlock_project(&project_id, Some(&other_job), &context),
            Ok(UnlockResult::LockedByOtherJob)
        ));
        assert!(matches!(is_project_locked(&project_id), Ok(true)));
        assert!(matches!(
            unlock_project(&project_id, Some(&job), &context),
            Ok(UnlockResult::Unlocked)
        ));
        assert_eq!(current_lock(&project_id), None);

        // unlocking can be repeated
        assert!(matches!(
            unlock_project(&project_id, Some(&job), &context),
            Ok(UnlockResult::Unlocked)
        ));

        hard_delete_project(&project_id);
    }

    #[test]
    #[serial]
    fn test_unlock_project_without_job() {
        let project_id = add_lock_test_project();
        let context = lock_context(ProjectRole::Member);
        let job = Uuid::new_v4();

        assert!(matches!(
            lock_project(&project_id, &job, "restore", &context),
            Ok(LockResult::Locked)
        ));
        // without a job every lock is removed, like by an admin after a failed restore
        assert!(matches!(
            unlock_project(&project_id, None, &context),
            Ok(UnlockResult::Unlocked)
        ));
        assert!(matches!(is_project_locked(&project_id), Ok(false)));

        // afterwards another job can lock it
        assert!(matches!(
            lock_project(&project_id, &Uuid::new_v4(), "backup", &context),
            Ok(LockResult::Locked)
        ));

        hard_delete_project(&project_id);
    }

    #[test]
    #[serial]
    fn test_admin_lock_blocks_jobs() {
        let project_id = add_lock_test_project();
        let context = lock_context(ProjectRole::Member);
        let admin_lock = Uuid::nil();
        let job = Uuid::new_v4();

        assert!(matches!(
            lock_project(&project_id, &admin_lock, "maintenance by admin", &context),
            Ok(LockResult::Locked)
        ));
        assert_eq!(
            current_lock(&project_id).unwrap().lock_reason,
            "maintenance by admin"
        );

        // locking it again as admin is accepted
        assert!(matches!(
            lock_project(&project_id, &admin_lock, "maintenance by admin", &context),
            Ok(LockResult::Locked)
        ));

        // no backup or restore can lock or unlock the project in the meantime
        assert!(matches!(
            lock_project(&project_id, &job, "backup", &context),
            Ok(LockResult::LockedByOtherJob)
        ));
        assert!(matches!(
            unlock_project(&project_id, Some(&job), &context),
            Ok(UnlockResult::LockedByOtherJob)
        ));

        // only the unlock without job removes it
        assert!(matches!(
            unlock_project(&project_id, None, &context),
            Ok(UnlockResult::Unlocked)
        ));
        assert!(matches!(is_project_locked(&project_id), Ok(false)));

        hard_delete_project(&project_id);
    }

    #[test]
    #[serial]
    fn test_lock_project_errors() {
        let project_id = add_lock_test_project();
        let job = Uuid::new_v4();

        // observers can't lock or unlock
        let observer = lock_context(ProjectRole::Observer);
        assert!(matches!(
            lock_project(&project_id, &job, "backup", &observer),
            Err(enums::DbError::PermissionDenied)
        ));
        assert!(matches!(
            unlock_project(&project_id, None, &observer),
            Err(enums::DbError::PermissionDenied)
        ));

        // unknown or deleted projects
        let context = lock_context(ProjectRole::Member);
        let unknown = "test-lock-unknown".to_string();
        assert!(matches!(
            lock_project(&unknown, &job, "backup", &context),
            Err(enums::DbError::NotFound)
        ));
        assert!(matches!(
            unlock_project(&unknown, Some(&job), &context),
            Err(enums::DbError::NotFound)
        ));
        assert!(matches!(
            is_project_locked(&unknown),
            Err(enums::DbError::NotFound)
        ));

        // an incomplete lock in the database is reported
        let mut project = get_auth_project(&project_id).ok().unwrap();
        project.locked_by_job = Some(job);
        assert!(lock_of(&project).is_err());
        project.lock_reason = Some("any description".to_string());
        assert!(lock_of(&project).is_err());
        project.locked_at = Some(Utc::now());
        assert_eq!(
            lock_of(&project).unwrap().unwrap().lock_reason,
            "any description"
        );

        hard_delete_project(&project_id);
    }
}
