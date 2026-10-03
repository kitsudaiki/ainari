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

use crate::database::db_handle;
use crate::database::quota_table;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the projects table
table! {
    projects (id) {
        id -> Varchar,
        name -> Varchar,
        status -> Varchar,
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
    quota_table::add_new_quota(project_id, 10, 10, 10, 10, 10, context)?;

    let project = ProjectEntry {
        id: project_id.clone(),
        name: project_name.to_owned(),
        status: "ACTIVE".to_string(),
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
}
