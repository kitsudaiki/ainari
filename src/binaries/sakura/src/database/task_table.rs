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
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

table! {
    tasks (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        model_uuid -> Varchar,
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

/// Represents a single task entry in the database.
///
/// This struct maps directly to the `tasks` table in the database and contains all the fields
/// necessary to track a task's progress, state, and metadata.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = tasks)]
pub struct TaskEntry {
    pub uuid: String,
    pub name: String,
    pub model_uuid: String,
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

/// Initializes the tasks table in the database if it doesn't already exist.
///
/// This function creates the tasks table with the appropriate schema. It's typically called
/// during application startup to ensure the required database tables exist.
///
/// # Returns
/// * `Ok(())` if the table was successfully initialized or already exists
/// * An error if there was a problem executing the SQL statement
pub fn init_task_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS tasks (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        model_uuid VARCHAR(40),
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

/// Adds a new task to the database.
///
/// This function creates a new task entry with the provided parameters and stores it in the database.
/// The task is initialized with the `Created` state and default values for progress fields.
///
/// # Arguments
/// * `task_uuid` - Unique identifier for the task
/// * `model_uuid` - Identifier for the associated model
/// * `task_name` - Name of the task
/// * `task_type` - Type of the task
/// * `total_number_of_epochs` - Total number of epochs the task should run
/// * `total_number_of_cycles` - Total number of cycles the task should run
/// * `context` - User context containing user ID and project ID
///
/// # Returns
/// * `QueryResult<usize>` - Number of rows affected by the insert operation
pub fn add_new_task(
    task_uuid: &Uuid,
    model_uuid: &Uuid,
    task_name: &str,
    task_type: &TaskType,
    total_number_of_epochs: &u64,
    total_number_of_cycles: &u64,
    context: &UserContext,
) -> QueryResult<usize> {
    // Create a new TaskEntry with the provided parameters
    let task = TaskEntry {
        uuid: task_uuid.to_string().clone(),
        name: task_name.to_owned(),
        model_uuid: model_uuid.to_string().clone(),
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

    // Insert the task into the database
    add_task(&task)
}

/// Internal function to add a task to the database.
///
/// This function handles the actual database insertion of a TaskEntry.
///
/// # Arguments
/// * `task` - The task to be inserted
///
/// # Returns
/// * `QueryResult<usize>` - Number of rows affected by the insert operation
fn add_task(task: &TaskEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    diesel::insert_into(tasks).values(task).execute(&mut *conn)
}

/// Retrieves a specific task from the database.
///
/// This function fetches a task by its UUID and model UUID, applying appropriate access control
/// based on the user's permissions in the provided context.
///
/// # Arguments
/// * `task_uuid` - UUID of the task to retrieve
/// * `model_uuid_in` - UUID of the associated model
/// * `context` - User context containing user ID, project ID, and admin status
///
/// # Returns
/// * `Result<TaskEntry, enums::DbError>` - The requested task or an error if not found or other error occurs
pub fn get_task(
    task_uuid: &Uuid,
    model_uuid_in: &Uuid,
    context: &UserContext,
) -> Result<TaskEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Start building the query with the required filters
    let mut query = tasks
        // HINT (kitsudaiki): Had to rename the function-parameter model_uuid to model_uuid_in to have a different name,
        // because here in this filter, it results in conflicts in case both sides of the eq are named the same
        .filter(
            uuid.eq(task_uuid.to_string())
                .and(model_uuid.eq(model_uuid_in.to_string())),
        )
        .into_boxed();

    // Apply access control filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and handle the result
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

/// Lists all tasks associated with a specific model in the database.
///
/// This function retrieves all tasks for a given model UUID, applying appropriate access control
/// based on the user's permissions in the provided context.
///
/// # Arguments
/// * `model_uuid_in` - UUID of the model to list tasks for
/// * `context` - User context containing user ID, project ID, and admin status
///
/// # Returns
/// * `QueryResult<Vec<TaskEntry>>` - Vector of task entries or an error if one occurs
pub fn list_tasks(model_uuid_in: &Uuid, context: &UserContext) -> QueryResult<Vec<TaskEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Start building the query with the required filters
    let mut query = tasks
        .filter(model_uuid.eq(model_uuid_in.to_string()))
        .into_boxed();

    // Apply access control filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    // Execute the query and return the results
    query.select(TaskEntry::as_select()).load(&mut *conn)
}

/// Updates the progress of a task in the database.
///
/// This function updates the current epoch and cycle counters for a specific task.
///
/// # Arguments
/// * `task_uuid` - UUID of the task to update
/// * `epoch` - New value for current_epoch
/// * `cycle` - New value for current_cycle
///
/// # Returns
/// * `Result<(), ()>` - Ok(()) if successful, Err(()) if the task was not found or another error occurred
pub fn update_task_progress(task_uuid: &Uuid, epoch: &i64, cycle: &i64) -> Result<(), ()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Update the task's progress fields
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

/// Updates the state of a task in the database.
///
/// This function changes the state of a task and updates the appropriate timestamp fields
/// based on the new state.
///
/// # Arguments
/// * `task_uuid` - UUID of the task to update
/// * `new_state` - The new state to set for the task
///
/// # Returns
/// * `Result<(), enums::DbError>` - Ok(()) if successful, an error if the task was not found or another error occurred
pub fn update_task_state(task_uuid: &Uuid, new_state: &TaskState) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Handle different states with appropriate updates
    match new_state {
        TaskState::Created => Ok(()),
        TaskState::Queued => {
            // Update task state and set queued_at timestamp
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
            // Update task state and set started_at timestamp
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
            // Update task state and set aborted_at timestamp
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
            // Update task state and set finished_at timestamp
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

/// Sets an error state for a task in the database.
///
/// This function updates a task's state to Error and sets the error message.
///
/// # Arguments
/// * `task_uuid` - UUID of the task to update
/// * `error_msg` - Error message to store
///
/// # Returns
/// * `Result<(), ()>` - Ok(()) if successful, Err(()) if the task was not found or another error occurred
pub fn set_error_state(task_uuid: &Uuid, error_msg: &String) -> Result<(), ()> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Update task state to Error and set the error message
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

/// Checks if a task has been aborted.
///
/// This function queries the database to determine if a task is in the Aborted state.
///
/// # Arguments
/// * `task_uuid` - UUID of the task to check
///
/// # Returns
/// * `bool` - true if the task is aborted, false if not found or not aborted
pub fn is_aborted(task_uuid: &Uuid) -> bool {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Build and execute the query to get the task
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
        let model_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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
        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
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
        let model_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let task1 = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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
        let tasks = list_tasks(&model_uuid, &context).unwrap();
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
        let model_uuid = Uuid::new_v4();

        let task1 = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let tasks = list_tasks(&model_uuid, &context).unwrap();
        assert_eq!(tasks.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let tasks = list_tasks(&model_uuid, &context).unwrap();
        assert_eq!(tasks.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let tasks = list_tasks(&model_uuid, &context).unwrap();
        assert_eq!(tasks.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_task(&uuid1, &model_uuid, &context) {
            Ok(retrieved_task) => {
                assert_eq!(retrieved_task.uuid, uuid1.to_string());
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        // get-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        if get_task(&uuid3, &model_uuid, &context).is_ok() {
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
        let model_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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

        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Created.to_string());
            assert_eq!(retrieved_task.queued_at, None);
            assert_eq!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Queued);

        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Queued.to_string());
            assert_ne!(retrieved_task.queued_at, None);
            assert_eq!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Active);

        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Active.to_string());
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Aborted);

        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Aborted.to_string());
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_ne!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Finished);

        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
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
        let model_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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

        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
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
        let model_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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

        if let Ok(retrieved_task) = get_task(&uuid1, &model_uuid, &context) {
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
        let model_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();

        let task = TaskEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            model_uuid: model_uuid.to_string(),
            task_type: TaskType::Train.to_string(),
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
