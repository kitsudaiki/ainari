// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use chrono::Utc;
use diesel::connection::SimpleConnection;
use std::error::Error;

use crate::database::db_handle;
use crate::api::user_context::UserContext;

use hanami_common::enums;

// Define the schema
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

pub fn init_project_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    let _ = conn.batch_execute("CREATE TABLE IF NOT EXISTS projects (
        id VARCHAR(256),
        name VARCHAR(256),
        status VARCHAR(10),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(256)
    );")?;

    Ok(())
}

pub fn add_new_project(project_id: &String, project_name: &String, context: &UserContext) -> QueryResult<usize> {
    if context.is_admin == false {
        return Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            Box::new("Permission denied.".to_string())
        ))
    }

    // check if project alredy exist in the database
    // The same id is allowed multiple times in the table, but only one time active.
    match get_project(&project_id, &context) {
        Ok(_) => {
            return Err(diesel::result::Error::DatabaseError(
                DatabaseErrorKind::UniqueViolation,
                Box::new(format!("Project with ID '{project_id}' already exist."))
            ))
        },
        Err(_) => {}
    };

    let project = ProjectEntry{
        id: project_id.clone(),
        name: project_name.clone(),
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

pub fn add_project(project: &ProjectEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;

    diesel::insert_into(projects).values(project).execute(&mut *conn)
}

pub fn get_project(project_id: &String, context: &UserContext) -> Result<ProjectEntry, enums::DbError> {
    if context.is_admin == false {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;
    match projects
        .filter(id.eq(project_id).and(status.eq("ACTIVE")))
        .select(ProjectEntry::as_select())
        .first::<ProjectEntry>(&mut *conn)
    {
        Ok(project) => Ok(project),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {:?}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_projects(context: &UserContext) -> QueryResult<Vec<ProjectEntry>> {  
    if context.is_admin == false {
        let dummy: QueryResult<Vec<ProjectEntry>> = Ok(vec![]);
        return dummy;
    }

    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;
    projects.filter(status.eq("ACTIVE")).select(ProjectEntry::as_select()).load(&mut *conn)
}

pub fn delete_project(project_id: &String, context: &UserContext) -> Result<(), enums::DbError> {
    if context.is_admin == false {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;
    match diesel::update(projects.filter(id.eq(project_id)))
        .set(status.eq("DELETED"))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {:?}", e);
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
        let mut conn = db_handle::DB_CONN.lock().unwrap();
        let _ = diesel::delete(projects.filter(id.eq(project_id))).execute(&mut *conn);
    }
    
    #[test]
    #[serial]
    fn test_add_get_project() {
        let _ = init_project_table();
        let project_id = "test-project-1".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true,
            is_project_admin: false,
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
        match get_project(&project_id, &context) {
            Ok(retrieved_project) => {
                assert_eq!(retrieved_project.id, project.id);
                assert_eq!(retrieved_project.name, project.name);
                assert_eq!(retrieved_project.status, project.status);
                assert_eq!(retrieved_project.created_by, project.created_by);
                assert_eq!(retrieved_project.updated_by, project.updated_by);
                assert_eq!(retrieved_project.deleted_at, project.deleted_at);
                assert_eq!(retrieved_project.deleted_by, project.deleted_by);
            },
            Err(_) => {}
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
            user_id: owner_id.clone(),
            project_id: project_id1.clone(),
            is_admin: true,
            is_project_admin: false,
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
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true,
            is_project_admin: false,
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
