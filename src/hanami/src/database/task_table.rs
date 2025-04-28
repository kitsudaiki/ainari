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
use uuid::Uuid;

use crate::database::db_handle;
use crate::api::user_context::UserContext;

use hanami_common::functions::sha256_hash;
use hanami_common::enums;

// Define the schema
table! {
    tasks (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        task_type -> Varchar,
        owner_id -> Varchar,
        project_id -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
    }
}

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = tasks)]
pub struct task {
    pub uuid: String,
    pub name: String,
    pub task_type: String,
    pub owner_id: String,
    pub project_id: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
}

pub fn init_task_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    let _ = conn.batch_execute("CREATE TABLE IF NOT EXISTS tasks (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        task_type VARCHAR(32),
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256)
    );")?;

    Ok(())
}

pub fn add_new_task(task_uuid: &Uuid, task_name: &String, task_type: &String, context: &UserContext) -> QueryResult<usize> {
    let task = task{
        uuid: task_uuid.to_string().clone(),
        name: task_name.clone(),
        task_type: task_type.clone(),
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now().to_rfc3339(),
        updated_by: context.user_id.clone(),
    };

    add_task(&task)
}

fn add_task(task: &task) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::tasks::dsl::*;

    diesel::insert_into(tasks).values(task).execute(&mut *conn)
}

pub fn get_task(task_uuid: &Uuid, context: &UserContext) -> Result<task, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::tasks::dsl::*;
    
    let mut query = tasks
        .filter(uuid.eq(task_uuid.to_string()))
        .into_boxed();

    if context.is_admin == false {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin == false {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        } 
    }

    match query
        .select(task::as_select())
        .first::<task>(&mut *conn)
    {
        Ok(task) => Ok(task),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            error!("Database-error: {}", e);
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_tasks(context: &UserContext) -> QueryResult<Vec<task>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::tasks::dsl::*;

    let mut query = tasks.into_boxed();

    if context.is_admin == false {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin == false {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(task::as_select()).load(&mut *conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hard_delete_task(task_uuid: &Uuid) {
        use self::tasks::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().unwrap();
        let _ = diesel::delete(tasks.filter(uuid.eq(task_uuid.to_string()))).execute(&mut *conn);
    }
    
    #[test]
    fn test_add_get_task() {
        let _ = init_task_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let task: task = task {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            task_type: "Train-Task".to_string(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(&task).unwrap();
        match get_task(&uuid1, &context) {
            Ok(retrieved_task) => {
                assert_eq!(retrieved_task.uuid, task.uuid);
                assert_eq!(retrieved_task.name, task.name);
                assert_eq!(retrieved_task.created_by, task.created_by);
                assert_eq!(retrieved_task.updated_by, task.updated_by);
            },
            Err(_) => {}
        };

        let _ = hard_delete_task(&uuid1);
    }

    #[test]
    fn test_list_tasks() {
        let _ = init_task_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let task1 = task {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            task_type: "Train-Task".to_string(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };
        
        let task2 = task {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            task_type: "Train-Task".to_string(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };
        
        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);

        add_task(&task1).unwrap();
        add_task(&task2).unwrap();
        let tasks = list_tasks(&context).unwrap();
        assert_eq!(tasks.len(), 2);
        let _ = hard_delete_task(&uuid1);
        let _ = hard_delete_task(&uuid2);
    }

    #[test]
    fn test_tasks_permissions() {
        let _ = init_task_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        let task1 = task {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            task_type: "Train-Task".to_string(),
            owner_id: "test-user-1".to_string(),
            project_id: "test-project-1".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };
        
        let task2 = task {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            task_type: "Train-Task".to_string(),
            owner_id: "test-user-2".to_string(),
            project_id: "test-project-1".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };
                
        let task3 = task {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            task_type: "Train-Task".to_string(),
            owner_id: "test-user-3".to_string(),
            project_id: "test-project-2".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };
        
        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);
        hard_delete_task(&uuid3);

        add_task(&task1).unwrap();
        add_task(&task2).unwrap();
        add_task(&task3).unwrap();

        // list-test normal user
        let context = UserContext {
            user_id: "test-user-1".to_string(),
            project_id: "test-project-1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        let tasks = list_tasks(&context).unwrap();
        assert_eq!(tasks.len(), 1);

        // list-test project-admin
        let context = UserContext {
            user_id: "test-user-1".to_string(),
            project_id: "test-project-1".to_string(),
            is_admin: false,
            is_project_admin: true,
        };
        let tasks = list_tasks(&context).unwrap();
        assert_eq!(tasks.len(), 2);

        // list-test admin
        let context = UserContext {
            user_id: "test-user-1".to_string(),
            project_id: "test-project-1".to_string(),
            is_admin: true,
            is_project_admin: false,
        };
        let tasks = list_tasks(&context).unwrap();
        assert_eq!(tasks.len(), 3);

        // get-test normal user
        let context = UserContext {
            user_id: "test-user-1".to_string(),
            project_id: "test-project-1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        match get_task(&uuid1, &context) {
            Ok(retrieved_task) => {
                assert_eq!(retrieved_task.uuid, uuid1.to_string());
            },
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        // get-test normal user false uuid
        let context = UserContext {
            user_id: "test-user-1".to_string(),
            project_id: "test-project-1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        match get_task(&uuid3, &context) {
            Ok(_) => {
                assert_eq!(true, false);
            },
            Err(_) => {}
        };

        let _ = hard_delete_task(&uuid1);
        let _ = hard_delete_task(&uuid2);
        let _ = hard_delete_task(&uuid3);
    }
}
