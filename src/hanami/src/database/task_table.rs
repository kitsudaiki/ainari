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
use hanami_common::functions::sha256_hash;
use hanami_common::enums;

// Define the schema
table! {
    tasks (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        task_type -> Varchar,
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
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256)
    );")?;

    Ok(())
}

pub fn add_new_task(task_uuid: &Uuid, task_name: &String, task_type: &String, creator_id: &String) -> QueryResult<usize> {
    let task = task{
        uuid: task_uuid.to_string().clone(),
        name: task_name.clone(),
        task_type: task_type.clone(),
        created_at: "".to_string(),
        created_by: creator_id.clone(),
        updated_at: "".to_string(),
        updated_by: creator_id.clone(),
    };

    add_task(&task)
}

fn add_task(task: &task) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::tasks::dsl::*;

    let mut new_task = task.clone();
    new_task.created_at = Utc::now().to_rfc3339();
    new_task.updated_at = Utc::now().to_rfc3339();

    diesel::insert_into(tasks).values(new_task).execute(&mut *conn)
}

pub fn get_task(task_uuid: &Uuid) -> Result<task, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::tasks::dsl::*;
    match tasks
        .filter(uuid.eq(task_uuid.to_string()))
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

pub fn list_tasks() -> QueryResult<Vec<task>> {
    let mut conn = db_handle::DB_CONN.lock().unwrap();
    use self::tasks::dsl::*;
    tasks.select(task::as_select()).load(&mut *conn)
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

        let task: task = task {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            task_type: "Train-Task".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(&task).unwrap();
        match get_task(&uuid1) {
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

        let task1 = task {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            task_type: "Train-Task".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };
        
        let task2 = task {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            task_type: "Train-Task".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
        };
        
        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);

        add_task(&task1).unwrap();
        add_task(&task2).unwrap();
        let tasks = list_tasks().unwrap();
        assert_eq!(tasks.len(), 2);
        let _ = hard_delete_task(&uuid1);
        let _ = hard_delete_task(&uuid2);
    }
}
