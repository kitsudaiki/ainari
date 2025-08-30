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

use chrono::Utc;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;
use ainari_api::user_context::UserContext;

use ainari_api::structs::task_structs::{TaskState, TaskType};
use ainari_common::enums;

// Define the schema
table! {
    tasks (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        cluster_uuid -> Varchar,
        task_type -> Varchar,
        task_state -> Varchar,
        total_number_of_epochs -> BigInt,
        current_epoch -> BigInt,
        total_number_of_cycles -> BigInt,
        current_cycle -> BigInt,
        queued_at -> Nullable<Varchar>,
        started_at -> Nullable<Varchar>,
        aborted_at -> Nullable<Varchar>,
        finished_at -> Nullable<Varchar>,
        error_message -> Nullable<Text>,
        owner_id -> Varchar,
        project_id -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
    }
}

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = tasks)]
pub struct TaskEntry {
    pub uuid: String,
    pub name: String,
    pub cluster_uuid: String,
    pub task_type: String,
    pub task_state: String,
    pub total_number_of_epochs: i64,
    pub current_epoch: i64,
    pub total_number_of_cycles: i64,
    pub current_cycle: i64,
    pub queued_at: Option<String>,
    pub started_at: Option<String>,
    pub aborted_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_message: Option<String>,
    pub owner_id: String,
    pub project_id: String,
    pub created_at: String,
    pub created_by: String,
}

pub fn init_task_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS tasks (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        cluster_uuid VARCHAR(40),
        task_type VARCHAR(32),
        task_state VARCHAR(32),
        total_number_of_epochs INTEGER,
        current_epoch INTEGER,
        total_number_of_cycles INTEGER,
        current_cycle INTEGER,
        queued_at VARCHAR(64),
        started_at VARCHAR(64),
        aborted_at VARCHAR(64),
        finished_at VARCHAR(64),
        error_message TEXT,
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        created_at VARCHAR(64),
        created_by VARCHAR(256)
    );",
    )?;

    Ok(())
}

pub fn add_new_task(
    task_uuid: &Uuid,
    cluster_uuid: &Uuid,
    task_name: &str,
    task_type: &TaskType,
    total_number_of_epochs: &u64,
    total_number_of_cycles: &u64,
    context: &UserContext,
) -> QueryResult<usize> {
    let task = TaskEntry {
        uuid: task_uuid.to_string().clone(),
        name: task_name.to_owned(),
        cluster_uuid: cluster_uuid.to_string().clone(),
        task_type: task_type.to_string(),
        task_state: TaskState::Created.to_string(),
        total_number_of_epochs: *total_number_of_epochs as i64,
        current_epoch: 0,
        total_number_of_cycles: *total_number_of_cycles as i64,
        current_cycle: 0,
        queued_at: None,
        started_at: None,
        aborted_at: None,
        finished_at: None,
        error_message: None,
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
    };

    add_task(&task)
}

fn add_task(task: &TaskEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    diesel::insert_into(tasks).values(task).execute(&mut *conn)
}

pub fn get_task(
    task_uuid: &Uuid,
    cluster_uuid_in: &Uuid,
    context: &UserContext,
) -> Result<TaskEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    let mut query = tasks
        // HINT (kitsudaiki): Had to rename the function-parameter cluster_uuid to cluster_uuid_in to have a different name,
        // because here in this filter, it results in conflicts in case both sides of the eq are named the same
        .filter(
            uuid.eq(task_uuid.to_string())
                .and(cluster_uuid.eq(cluster_uuid_in.to_string())),
        )
        .into_boxed();

    if !context.is_admin {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if !context.is_project_admin {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(TaskEntry::as_select())
        .first::<TaskEntry>(&mut *conn)
    {
        Ok(task) => Ok(task),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_tasks(cluster_uuid_in: &Uuid, context: &UserContext) -> QueryResult<Vec<TaskEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    let mut query = tasks
        .filter(cluster_uuid.eq(cluster_uuid_in.to_string()))
        .into_boxed();

    if !context.is_admin {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if !context.is_project_admin {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(TaskEntry::as_select()).load(&mut *conn)
}

pub fn update_task_progress(task_uuid: &Uuid, epoch: &i64, cycle: &i64) -> Result<(), ()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    match diesel::update(tasks.filter(uuid.eq(task_uuid.to_string())))
        .set((current_epoch.eq(epoch), current_cycle.eq(cycle)))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(())
        }
    }
}

pub fn update_task_state(task_uuid: &Uuid, new_state: &TaskState) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    match new_state {
        TaskState::Created => Ok(()),
        TaskState::Queued => {
            match diesel::update(tasks.filter(uuid.eq(task_uuid.to_string())))
                .set((
                    task_state.eq(new_state.to_string()),
                    queued_at.eq(Utc::now().to_rfc3339()),
                ))
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
        TaskState::Active => {
            match diesel::update(tasks.filter(uuid.eq(task_uuid.to_string())))
                .set((
                    task_state.eq(new_state.to_string()),
                    started_at.eq(Utc::now().to_rfc3339()),
                ))
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
        TaskState::Aborted => {
            match diesel::update(tasks.filter(uuid.eq(task_uuid.to_string())))
                .set((
                    task_state.eq(new_state.to_string()),
                    aborted_at.eq(Utc::now().to_rfc3339()),
                ))
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
        TaskState::Finished => {
            match diesel::update(tasks.filter(uuid.eq(task_uuid.to_string())))
                .set((
                    task_state.eq(new_state.to_string()),
                    finished_at.eq(Utc::now().to_rfc3339()),
                ))
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
        TaskState::Error => Ok(()),
    }
}

pub fn set_error_state(task_uuid: &Uuid, error_msg: &String) -> Result<(), ()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    match diesel::update(tasks.filter(uuid.eq(task_uuid.to_string())))
        .set((
            task_state.eq(TaskState::Error.to_string()),
            error_message.eq(error_msg),
        ))
        .execute(&mut *conn)
    {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(())
        }
    }
}

pub fn is_aborted(task_uuid: &Uuid) -> bool {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    let query = tasks.filter(uuid.eq(task_uuid.to_string())).into_boxed();

    match query
        .select(TaskEntry::as_select())
        .first::<TaskEntry>(&mut *conn)
    {
        Ok(task) => task.task_state == TaskState::Aborted.to_string(),
        Err(diesel::result::Error::NotFound) => false,
        Err(e) => {
            log::error!("Database-error: {e:?}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn hard_delete_task(task_uuid: &Uuid) {
        use self::tasks::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(tasks.filter(uuid.eq(task_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_task() {
        let _ = init_task_table();
        let uuid1 = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(&task).unwrap();
        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.uuid, task.uuid);
            assert_eq!(retrieved_task.name, task.name);
            assert_eq!(retrieved_task.created_by, task.created_by);
        };

        hard_delete_task(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_tasks() {
        let _ = init_task_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let task1 = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        let task2 = TaskEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);

        add_task(&task1).unwrap();
        add_task(&task2).unwrap();
        let tasks = list_tasks(&cluster_uuid, &context).unwrap();
        assert_eq!(tasks.len(), 2);
        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);
    }

    #[test]
    #[serial]
    fn test_tasks_permissions() {
        let _ = init_task_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();

        let task1 = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        let task2 = TaskEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        let task3 = TaskEntry {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);
        hard_delete_task(&uuid3);

        add_task(&task1).unwrap();
        add_task(&task2).unwrap();
        add_task(&task3).unwrap();

        // list-test normal user
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        let tasks = list_tasks(&cluster_uuid, &context).unwrap();
        assert_eq!(tasks.len(), 1);

        // list-test project-admin
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: true,
        };
        let tasks = list_tasks(&cluster_uuid, &context).unwrap();
        assert_eq!(tasks.len(), 2);

        // list-test admin
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true,
            is_project_admin: false,
        };
        let tasks = list_tasks(&cluster_uuid, &context).unwrap();
        assert_eq!(tasks.len(), 3);

        // get-test normal user
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        match get_task(&uuid1, &cluster_uuid, &context) {
            Ok(retrieved_task) => {
                assert_eq!(retrieved_task.uuid, uuid1.to_string());
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        // get-test normal user false uuid
        let context = UserContext {
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        if get_task(&uuid3, &cluster_uuid, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);
        hard_delete_task(&uuid3);
    }

    #[test]
    #[serial]
    fn test_update_task_state() {
        init_task_table().unwrap();
        let uuid1 = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(&task).unwrap();

        let _ = update_task_state(&uuid1, &TaskState::Created);

        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Created.to_string());
            assert_eq!(retrieved_task.queued_at, None);
            assert_eq!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Queued);

        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Queued.to_string());
            assert_ne!(retrieved_task.queued_at, None);
            assert_eq!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Active);

        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Active.to_string());
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Aborted);

        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Aborted.to_string());
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_ne!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Finished);

        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Finished.to_string());
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_ne!(retrieved_task.aborted_at, None);
            assert_ne!(retrieved_task.finished_at, None);
        };

        hard_delete_task(&uuid1);
    }

    #[test]
    #[serial]
    fn test_update_task_progress() {
        init_task_table().unwrap();
        let uuid1 = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(&task).unwrap();

        update_task_progress(&uuid1, &123, &42).unwrap();

        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.current_cycle, 42);
            assert_eq!(retrieved_task.current_epoch, 123);
        };

        hard_delete_task(&uuid1);
    }

    #[test]
    #[serial]
    fn test_set_error_state() {
        init_task_table().unwrap();
        let uuid1 = Uuid::new_v4();
        let error_msg = "This is an error".to_string();
        let cluster_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(&task).unwrap();

        update_task_progress(&uuid1, &123, &42).unwrap();

        let _ = set_error_state(&uuid1, &error_msg);

        if let Ok(retrieved_task) = get_task(&uuid1, &cluster_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Error.to_string());
            assert_eq!(retrieved_task.error_message, Some(error_msg));
        };

        hard_delete_task(&uuid1);
    }

    #[test]
    #[serial]
    fn test_is_aborted() {
        init_task_table().unwrap();
        let uuid1 = Uuid::new_v4();
        let cluster_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            cluster_uuid: cluster_uuid.to_string(),
            task_type: TaskType::TrainTask.to_string(),
            task_state: TaskState::Created.to_string(),
            total_number_of_epochs: 42,
            current_epoch: 0,
            total_number_of_cycles: 43,
            current_cycle: 0,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            error_message: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(&task).unwrap();

        assert!(!is_aborted(&uuid1));

        let _ = update_task_state(&uuid1, &TaskState::Aborted);

        assert!(is_aborted(&uuid1));

        hard_delete_task(&uuid1);
    }
}
