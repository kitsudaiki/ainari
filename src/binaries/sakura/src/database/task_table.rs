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
use diesel::Connection; // Required for .transaction()
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::task_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

table! {
    tasks (uuid) {
        uuid -> Varchar,
        description -> Varchar,
        resource_uuid -> Varchar,
        resource_type -> Varchar,
        task_type -> Varchar,
        task_state -> Varchar,
        queued_at -> Nullable<Varchar>,
        started_at -> Nullable<Varchar>,
        aborted_at -> Nullable<Varchar>,
        finished_at -> Nullable<Varchar>,
        messages -> Text,
        owner_id -> Varchar,
        project_id -> Varchar,
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
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub description: String,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub resource_uuid: Uuid,
    pub resource_type: String,
    pub task_type: TaskType,
    pub task_state: TaskState,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub queued_at: Option<DateTime<Utc>>,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub started_at: Option<DateTime<Utc>>,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub aborted_at: Option<DateTime<Utc>>,
    #[diesel(serialize_as = DbOptDateTime, deserialize_as = DbOptDateTime)]
    pub finished_at: Option<DateTime<Utc>>,
    #[diesel(serialize_as = DbVecString, deserialize_as = DbVecString)]
    pub messages: Vec<String>,
    pub owner_id: String,
    pub project_id: String,
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
        description VARCHAR(256),
        resource_uuid VARCHAR(40),
        resource_type VARCHAR(32),
        task_type VARCHAR(32),
        task_state VARCHAR(32),
        queued_at VARCHAR(64),
        started_at VARCHAR(64),
        aborted_at VARCHAR(64),
        finished_at VARCHAR(64),
        messages TEXT,
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        created_by VARCHAR(256)
    );",
    )?;

    // renamed separately, so it is also renamed in tables of older versions. In new tables the
    // old column doesn't exist, so the error for the missing column is ignored.
    match conn.batch_execute("ALTER TABLE tasks RENAME COLUMN name TO description;") {
        Ok(()) => {}
        Err(e) if e.to_string().contains("no such column") => {}
        Err(e) => return Err(e.into()),
    }

    Ok(())
}

/// Adds a new task to the database.
///
/// This function creates a new task entry with the provided parameters and stores it in the database.
/// The task is initialized with the `Created` state and default values for progress fields.
///
/// # Arguments
/// * `task_uuid` - Unique identifier for the task
/// * `resource_uuid` - Identifier for the associated resource
/// * `resource_type` - Type of the associated resource
/// * `task_description` - Human-readable description of the task
/// * `task_type` - Type of the task
/// * `context` - User context containing user ID and project ID
///
/// # Returns
/// * `QueryResult<usize>` - Number of rows affected by the insert operation
pub fn add_new_task(
    task_uuid: &Uuid,
    resource_uuid: &Uuid,
    resource_type: &TaskResourceType,
    task_description: &str,
    task_type: &TaskType,
    context: &UserContext,
) -> QueryResult<usize> {
    // Create a new TaskEntry with the provided parameters
    let task = TaskEntry {
        uuid: *task_uuid,
        description: task_description.to_owned(),
        resource_uuid: *resource_uuid,
        resource_type: resource_type.to_string().clone(),
        task_type: task_type.clone(),
        task_state: TaskState::Created,
        queued_at: None,
        started_at: None,
        aborted_at: None,
        finished_at: None,
        messages: Vec::new(),
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        created_by: context.user_id.clone(),
    };

    // Insert the task into the database
    add_task(task)
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
fn add_task(task: TaskEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    diesel::insert_into(tasks).values(task).execute(&mut *conn)
}

/// Retrieves a specific task from the database.
///
/// This function fetches a task by its UUID, applying appropriate access control
/// based on the user's permissions in the provided context.
///
/// # Arguments
/// * `task_uuid` - UUID of the task to retrieve
/// * `context` - User context containing user ID, project ID, and admin status
///
/// # Returns
/// * `Result<TaskEntry, enums::DbError>` - The requested task or an error if not found or other error occurs
pub fn get_task(task_uuid: &Uuid, context: &UserContext) -> Result<TaskEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Start building the query with the required filters
    let mut query = tasks.filter(uuid.eq(task_uuid.to_string())).into_boxed();

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

/// Lists all tasks in the database.
///
/// This function retrieves all tasks, applying appropriate access control
/// based on the user's permissions in the provided context.
///
/// # Arguments
/// * `context` - User context containing user ID, project ID, and admin status
/// * `filter_resource_uuid` - If set, only the tasks of this resource are listed
///
/// # Returns
/// * `QueryResult<Vec<TaskEntry>>` - Vector of task entries or an error if one occurs
pub fn list_tasks(
    context: &UserContext,
    filter_resource_uuid: Option<&Uuid>,
) -> QueryResult<Vec<TaskEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Start building the query, which is only restricted by the access control
    let mut query = tasks.into_boxed();

    // Apply access control filters based on user permissions
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    if let Some(filter_uuid) = filter_resource_uuid {
        query = query.filter(resource_uuid.eq(filter_uuid.to_string()));
    }

    // Execute the query and return the results
    query.select(TaskEntry::as_select()).load(&mut *conn)
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

    // Execute everything inside a transaction
    let result = conn.transaction::<_, diesel::result::Error, _>(|transaction_conn| {
        // Pre-calculate target and values to keep the match arms clean
        let target = tasks.filter(uuid.eq(task_uuid.to_string()));
        let state_str = new_state.to_string();
        let now = Utc::now().to_rfc3339();

        // Handle different states with appropriate updates
        match new_state {
            TaskState::Created => {
                // No database update required for this state
                Ok(0)
            }
            // a failed task ended too, so it also gets the time of its end
            TaskState::Error => diesel::update(target)
                .set((task_state.eq(state_str), finished_at.eq(now)))
                .execute(transaction_conn),
            TaskState::Queued => diesel::update(target)
                .set((task_state.eq(state_str), queued_at.eq(now)))
                .execute(transaction_conn),
            TaskState::Active => diesel::update(target)
                .set((task_state.eq(state_str), started_at.eq(now)))
                .execute(transaction_conn),
            TaskState::Aborted => diesel::update(target)
                .set((task_state.eq(state_str), aborted_at.eq(now)))
                .execute(transaction_conn),
            TaskState::Finished => diesel::update(target)
                .set((task_state.eq(state_str), finished_at.eq(now)))
                .execute(transaction_conn),
        }
    });

    // Handle the result from the transaction once, cleanly
    match result {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error updating task state: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Appends a new message to the task's messages list.
pub fn add_message_to_task(task_uuid: &Uuid, new_message: &str) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::tasks::dsl::*;

    // Run inside a transaction so if anything fails, the database remains untouched
    let result = conn.transaction::<_, diesel::result::Error, _>(|transaction_conn| {
        // Fetch the current task entry
        let mut task: TaskEntry = tasks
            .filter(uuid.eq(task_uuid.to_string()))
            .first::<TaskEntry>(transaction_conn)?;

        // Append the new message to our native Rust Vec<String>
        task.messages.push(new_message.to_string());

        // Save only the updated messages column back to the database.
        // NOTE: Because `serialize_as` applies to the Struct during inserts,
        // when updating a single column directly, we must manually wrap it in DbVecString.
        diesel::update(tasks.filter(uuid.eq(task_uuid.to_string())))
            .set(messages.eq(DbVecString::from(task.messages)))
            .execute(transaction_conn)?;

        Ok(())
    });

    // Handle the result mapping to your custom DbError enum
    match result {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error updating messages: {e:?}");
            Err(enums::DbError::InternalError)
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
        Ok(task) => task.task_state == TaskState::Aborted,
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
        let resource_uuid = Uuid::new_v4();
        let resource_type = TaskResourceType::VirtualMachine;

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
            uuid: uuid1,
            description: "Alice".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(task.clone()).unwrap();
        if let Ok(retrieved_task) = get_task(&uuid1, &context) {
            assert_eq!(retrieved_task.uuid, task.uuid);
            assert_eq!(retrieved_task.description, task.description);
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
        let resource_uuid = Uuid::new_v4();
        let other_resource_uuid = Uuid::new_v4();
        let resource_type = TaskResourceType::VirtualMachine;

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
            uuid: uuid1,
            description: "Alice".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_by: "admin".to_string(),
        };

        let task2 = TaskEntry {
            uuid: uuid2,
            description: "Bob".to_string(),
            resource_uuid: other_resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);

        add_task(task1).unwrap();
        add_task(task2).unwrap();
        let tasks = list_tasks(&context, None).unwrap();
        assert_eq!(tasks.len(), 2);

        // only the tasks of the requested resource
        let tasks = list_tasks(&context, Some(&resource_uuid)).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].uuid, uuid1);
        let tasks = list_tasks(&context, Some(&Uuid::new_v4())).unwrap();
        assert_eq!(tasks.len(), 0);

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
        let resource_uuid = Uuid::new_v4();
        let resource_type = TaskResourceType::VirtualMachine;

        let task1 = TaskEntry {
            uuid: uuid1,
            description: "Alice".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_by: "admin".to_string(),
        };

        let task2 = TaskEntry {
            uuid: uuid2,
            description: "Bob".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_by: "admin".to_string(),
        };

        let task3 = TaskEntry {
            uuid: uuid3,
            description: "Poi".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);
        hard_delete_task(&uuid2);
        hard_delete_task(&uuid3);

        add_task(task1).unwrap();
        add_task(task2).unwrap();
        add_task(task3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let tasks = list_tasks(&context, None).unwrap();
        assert_eq!(tasks.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let tasks = list_tasks(&context, None).unwrap();
        assert_eq!(tasks.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let tasks = list_tasks(&context, None).unwrap();
        assert_eq!(tasks.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_task(&uuid1, &context) {
            Ok(retrieved_task) => {
                assert_eq!(retrieved_task.uuid, uuid1);
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
        if get_task(&uuid3, &context).is_ok() {
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
        let resource_uuid = Uuid::new_v4();
        let resource_type = TaskResourceType::VirtualMachine;

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
            uuid: uuid1,
            description: "Alice".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(task).unwrap();

        let _ = update_task_state(&uuid1, &TaskState::Created);

        if let Ok(retrieved_task) = get_task(&uuid1, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Created);
            assert_eq!(retrieved_task.queued_at, None);
            assert_eq!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Queued);

        if let Ok(retrieved_task) = get_task(&uuid1, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Queued);
            assert_ne!(retrieved_task.queued_at, None);
            assert_eq!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Active);

        if let Ok(retrieved_task) = get_task(&uuid1, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Active);
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_eq!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Aborted);

        if let Ok(retrieved_task) = get_task(&uuid1, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Aborted);
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_ne!(retrieved_task.aborted_at, None);
            assert_eq!(retrieved_task.finished_at, None);
        };

        let _ = update_task_state(&uuid1, &TaskState::Finished);

        if let Ok(retrieved_task) = get_task(&uuid1, &context) {
            assert_eq!(retrieved_task.task_state, TaskState::Finished);
            assert_ne!(retrieved_task.queued_at, None);
            assert_ne!(retrieved_task.started_at, None);
            assert_ne!(retrieved_task.aborted_at, None);
            assert_ne!(retrieved_task.finished_at, None);
        };

        hard_delete_task(&uuid1);
    }

    #[test]
    #[serial]
    fn test_add_message_to_task() {
        init_task_table().unwrap();
        let uuid1 = Uuid::new_v4();
        let error_msg = "This is an error".to_string();
        let resource_uuid = Uuid::new_v4();
        let resource_type = TaskResourceType::VirtualMachine;

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
            uuid: uuid1,
            description: "Alice".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(task).unwrap();

        let _ = add_message_to_task(&uuid1, &error_msg);

        if let Ok(retrieved_task) = get_task(&uuid1, &context) {
            assert_eq!(retrieved_task.messages, vec![error_msg]);
        };

        hard_delete_task(&uuid1);
    }

    #[test]
    #[serial]
    fn test_is_aborted() {
        init_task_table().unwrap();
        let uuid1 = Uuid::new_v4();
        let resource_uuid = Uuid::new_v4();
        let resource_type = TaskResourceType::VirtualMachine;

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();

        let task = TaskEntry {
            uuid: uuid1,
            description: "Alice".to_string(),
            resource_uuid,
            resource_type: resource_type.to_string(),
            task_type: TaskType::VirtualMachineCreate,
            task_state: TaskState::Created,
            queued_at: None,
            started_at: None,
            aborted_at: None,
            finished_at: None,
            messages: Vec::new(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_by: "admin".to_string(),
        };

        hard_delete_task(&uuid1);

        add_task(task).unwrap();

        assert!(!is_aborted(&uuid1));

        let _ = update_task_state(&uuid1, &TaskState::Aborted);

        assert!(is_aborted(&uuid1));

        hard_delete_task(&uuid1);
    }
}
