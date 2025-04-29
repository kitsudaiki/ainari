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
use chrono::Utc;
use diesel::connection::SimpleConnection;
use log::{info, debug, error};
use std::env;
use std::error::Error;
use rand::{distr::Alphanumeric, Rng};

use crate::database::db_handle;
use hanami_common::functions::sha256_hash;
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
pub struct Project {
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
        id VARCHAR(256) PRIMARY KEY,
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

pub fn add_new_project(project_id: &String, project_name: &String, creator_id: &String) -> QueryResult<usize> {
    let project = Project{
        id: project_id.clone(),
        name: project_name.clone(),
        created_at: "".to_string(),
        created_by: creator_id.clone(),
        updated_at: "".to_string(),
        updated_by: creator_id.clone(),
        status: "".to_string(),
        deleted_at: None,
        deleted_by: None,
    };

    add_project(&project)
}

pub fn add_project(project: &Project) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;

    let mut new_project = project.clone();
    new_project.created_at = Utc::now().to_rfc3339();
    new_project.updated_at = Utc::now().to_rfc3339();
    new_project.status = "ACTIVE".to_string();

    diesel::insert_into(projects).values(new_project).execute(&mut *conn)
}

pub fn get_project(project_id: &String) -> Result<Project, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;
    match projects
        .filter(id.eq(project_id).and(status.eq("ACTIVE")))
        .select(Project::as_select())
        .first::<Project>(&mut *conn)
    {
        Ok(project) => Ok(project),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            error!("Database-error: {}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_projects() -> QueryResult<Vec<Project>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;
    projects.filter(status.eq("ACTIVE")).select(Project::as_select()).load(&mut *conn)
}

pub fn delete_project(project_id: &String) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::projects::dsl::*;
    match diesel::update(projects.filter(id.eq(project_id)))
        .set(status.eq("DELETED"))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            error!("Database-error: {}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hard_delete_project(project_id: &String) {
        use self::projects::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().unwrap();
        let _ = diesel::delete(projects.filter(id.eq(project_id))).execute(&mut *conn);
    }
    
    #[test]
    fn test_add_get_project() {
        let _ = init_project_table();
        let project: Project = Project {
            id: "1".to_string(),
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
        match get_project(&"1".to_string()) {
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

        let _ = delete_project(&project.id);
    }

    #[test]
    fn test_list_projects() {
        let _ = init_project_table();
        let project1 = Project {
            id: "2".to_string(),
            name: "Alice".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };
        
        let project2 = Project {
            id: "3".to_string(),
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
        let projects = list_projects().unwrap();
        assert_eq!(projects.len(), 2);
        let _ = delete_project(&project1.id);
        let _ = delete_project(&project2.id);
    }

    #[test]
    fn test_delete_project() {
        let _ = init_project_table();
        let project = Project {
            id: "4".to_string(),
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
        let _ = delete_project(&"4".to_string());
        let result = get_project(&"4".to_string());
        assert!(result.is_err());
    }
}
