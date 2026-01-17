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

use chrono::Utc;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use std::error::Error;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

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
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

/// Initializes the projects table in the database.
///
/// This function creates the projects table if it doesn't already exist.
/// It should be called during application startup.
///
/// # Returns
///
/// * `Ok(())` if the table was successfully initialized or already exists
/// * An error if the database operation fails
pub fn init_project_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS projects (
        id VARCHAR(256),
        name VARCHAR(256),
        status VARCHAR(8),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(256)
    );",
    )?;

    Ok(())
}

/// Adds a new project to the database.
///
/// This function creates a new project entry with the provided parameters.
/// It checks for admin permissions and ensures the project ID doesn't already exist.
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

    let project = ProjectEntry {
        id: project_id.clone(),
        name: project_name.to_owned(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now().to_rfc3339(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_project(&project)
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
pub fn add_project(project: &ProjectEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::projects::dsl::*;

    diesel::insert_into(projects)
        .values(project)
        .execute(&mut *conn)
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
    use serial_test::serial;

    fn hard_delete_project(project_id: &String) {
        use self::projects::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(projects.filter(id.eq(project_id))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_project() {
        let _ = init_project_table();
        let project_id = "test-project-1".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let project = ProjectEntry {
            id: project_id.clone(),
            name: "Alice".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_project(&project.id);

        add_project(&project).unwrap();
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
        let _ = init_project_table();
        let project_id1 = "test-project-2".to_string();
        let project_id2 = "test-project-3".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id1.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let project1 = ProjectEntry {
            id: project_id1.clone(),
            name: "Alice".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let project2 = ProjectEntry {
            id: project_id2.clone(),
            name: "Bob".to_string(),
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_project(&project1.id);
        hard_delete_project(&project2.id);

        add_project(&project1).unwrap();
        add_project(&project2).unwrap();

        let projects = list_projects(&context).unwrap();
        assert_eq!(projects.len(), 1);

        let _ = delete_project(&project1.id, &context);
        let _ = delete_project(&project2.id, &context);
    }

    #[test]
    #[serial]
    fn test_delete_project() {
        let _ = init_project_table();
        let project_id = "test-project-5".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let project = ProjectEntry {
            id: project_id.clone(),
            name: "Alice".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_project(&project.id);

        add_project(&project).unwrap();
        let _ = delete_project(&project_id, &context);
        let result = get_project(&project_id, &context);
        assert!(result.is_err());
    }
}
