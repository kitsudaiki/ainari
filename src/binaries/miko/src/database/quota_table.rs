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
use std::env;
use std::error::Error;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

// Define the schema for the quotas table
// This table stores quota information for users including maximum limits
// for various resources and their status.
table! {
    quotas (id) {
        id -> Varchar,
        max_model -> Integer,
        max_dataset -> Integer,
        max_checkpoint -> Integer,
        max_secret -> Integer,
        max_taskqueue -> Integer,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents a quota entry in the database.
///
/// This struct contains information about resource limits for a user
/// including maximum allowed models, datasets, checkpoints, secrets, and task queues.
/// It also tracks the status, creation, update, and deletion information.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = quotas)]
pub struct QuotaEntry {
    pub id: String,
    pub max_model: i32,
    pub max_dataset: i32,
    pub max_checkpoint: i32,
    pub max_secret: i32,
    pub max_taskqueue: i32,
    pub status: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

/// Initializes the quotas table in the database if it doesn't already exist.
///
/// This function creates the quotas table with the appropriate schema
/// and then calls `init_admin_quota` to set up the initial admin quota.
///
/// # Returns
/// - `Ok(())` if the table was created successfully or already exists
/// - An error if the table creation fails
pub fn init_quota_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS quotas (
        id VARCHAR(256),
        max_model INTEGER,
        max_dataset INTEGER,
        max_checkpoint INTEGER,
        max_secret INTEGER,
        max_taskqueue INTEGER,
        status VARCHAR(8),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(64)
    );",
    )?;
    // release lock on the connection to avoid dead-lock
    drop(conn);

    init_admin_quota()
}

/// Initializes the admin quota with default values.
///
/// This function checks if there are existing quotas in the database.
/// If none are found, it creates a new admin quota with default values
/// using the provided environment variable for the admin ID.
///
/// # Returns
/// - `Ok(())` if the admin quota was initialized successfully or already exists
/// - An error if the initialization fails or the environment variable is not found
pub fn init_admin_quota() -> Result<(), Box<dyn Error>> {
    let fake_admin_context = UserContext {
        token: "".to_string(),
        user_id: "AINARI_INIT".to_string(),
        project_id: "AINARI_INIT".to_string(),
        is_admin: true.to_string(),
        is_project_admin: false.to_string(),
    };

    let quotas = list_quotas(&fake_admin_context).unwrap();
    if !quotas.is_empty() {
        log::debug!("Already existing user found, so no new admin will be created.");
        return Ok(());
    }
    log::info!("No user found in user-table -> Create a new initial admin.");

    let admin_id: String = match env::var("AINARI_ADMIN_ID") {
        Ok(val) => val,
        Err(_) => {
            log::error!("couldn't find env-variable: AINARI_ADMIN_ID");
            return Err("An error occurred while initializing new admin-user".into());
        }
    };

    add_new_quota(&admin_id, 10, 10, 10, 10, 10, &fake_admin_context)?;

    Ok(())
}

/// Adds a new quota entry to the database.
///
/// This function creates a new quota entry with the provided parameters.
/// It checks if the user is an admin and if a quota already exists for the user ID.
///
/// # Arguments
/// * `user_id` - The ID of the user to create the quota for
/// * `max_model` - Maximum number of models allowed
/// * `max_dataset` - Maximum number of datasets allowed
/// * `max_checkpoint` - Maximum number of checkpoints allowed
/// * `max_secret` - Maximum number of secrets allowed
/// * `max_taskqueue` - Maximum number of task queues allowed
/// * `context` - The user context containing authentication information
///
/// # Returns
/// - `Ok(usize)` with the number of rows affected if successful
/// - An error if the user is not an admin, if the quota already exists, or if the insertion fails
pub fn add_new_quota(
    user_id: &String,
    max_model: i32,
    max_dataset: i32,
    max_checkpoint: i32,
    max_secret: i32,
    max_taskqueue: i32,
    context: &UserContext,
) -> QueryResult<usize> {
    if context.is_admin != true.to_string() {
        return Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            Box::new("Permission denied.".to_string()),
        ));
    }

    // check if quota already exists in the database
    // The same id is allowed multiple times in the table, but only one time active.
    if get_quota(user_id, context).is_ok() {
        return Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new(format!("User with ID '{user_id}' already exist.")),
        ));
    };

    let quota = QuotaEntry {
        id: user_id.clone(),
        max_model,
        max_dataset,
        max_checkpoint,
        max_secret,
        max_taskqueue,
        status: "ACTIVE".to_string(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now().to_rfc3339(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_quota(&quota)
}

/// Adds a quota entry to the database.
///
/// This function inserts a quota entry into the database.
/// It requires an active database connection.
///
/// # Arguments
/// * `quota` - The quota entry to insert
///
/// # Returns
/// - `Ok(usize)` with the number of rows affected if successful
/// - An error if the insertion fails
pub fn add_quota(quota: &QuotaEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::quotas::dsl::*;

    diesel::insert_into(quotas)
        .values(quota)
        .execute(&mut *conn)
}

/// Retrieves a quota entry from the database for a specific user.
///
/// This function queries the database for an active quota entry
/// associated with the provided user ID.
///
/// # Arguments
/// * `user_id` - The ID of the user to retrieve the quota for
/// * `context` - The user context containing authentication information
///
/// # Returns
/// - `Ok(QuotaEntry)` if the quota is found
/// - `enums::DbError::NotFound` if the quota is not found
/// - `enums::DbError::InternalError` if an error occurs while querying the database
pub fn get_quota(user_id: &String, _: &UserContext) -> Result<QuotaEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::quotas::dsl::*;
    match quotas
        .filter(id.eq(user_id).and(status.eq("ACTIVE")))
        .select(QuotaEntry::as_select())
        .first::<QuotaEntry>(&mut *conn)
    {
        Ok(quota) => Ok(quota),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all active quota entries in the database.
///
/// This function retrieves all quota entries with an "ACTIVE" status.
/// Only admin users can access this function.
///
/// # Arguments
/// * `context` - The user context containing authentication information
///
/// # Returns
/// - `Ok(Vec<QuotaEntry>)` with all active quota entries if successful
/// - An error if the query fails
pub fn list_quotas(context: &UserContext) -> QueryResult<Vec<QuotaEntry>> {
    if context.is_admin != true.to_string() {
        let dummy: QueryResult<Vec<QuotaEntry>> = Ok(vec![]);
        return dummy;
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::quotas::dsl::*;
    quotas
        .filter(status.eq("ACTIVE"))
        .select(QuotaEntry::as_select())
        .load(&mut *conn)
}

/// Updates an existing quota entry in the database.
///
/// This function modifies the resource limits for an existing quota entry.
/// Only admin users can access this function.
///
/// # Arguments
/// * `user_id` - The ID of the user to update the quota for
/// * `new_max_model` - New maximum number of models allowed
/// * `new_max_dataset` - New maximum number of datasets allowed
/// * `new_max_checkpoint` - New maximum number of checkpoints allowed
/// * `new_max_secret` - New maximum number of secrets allowed
/// * `new_max_taskqueue` - New maximum number of task queues allowed
/// * `context` - The user context containing authentication information
///
/// # Returns
/// - `Ok(())` if the quota was updated successfully
/// - `enums::DbError::NotFound` if the quota is not found
/// - `enums::DbError::InternalError` if an error occurs while updating the database
pub fn set_quota(
    user_id: &String,
    new_max_model: i32,
    new_max_dataset: i32,
    new_max_checkpoint: i32,
    new_max_secret: i32,
    new_max_taskqueue: i32,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    if context.is_admin != true.to_string() {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::quotas::dsl::*;

    match diesel::update(quotas.filter(id.eq(user_id.to_string())))
        .set((
            max_model.eq(new_max_model),
            max_dataset.eq(new_max_dataset),
            max_checkpoint.eq(new_max_checkpoint),
            max_secret.eq(new_max_secret),
            max_taskqueue.eq(new_max_taskqueue),
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

/// Marks a quota entry as deleted in the database.
///
/// This function updates the status of a quota entry to "DELETED".
/// Only admin users can access this function.
///
/// # Arguments
/// * `user_id` - The ID of the user to delete the quota for
/// * `context` - The user context containing authentication information
///
/// # Returns
/// - `Ok(())` if the quota was marked as deleted successfully
/// - `enums::DbError::NotFound` if the quota is not found
/// - `enums::DbError::InternalError` if an error occurs while updating the database
pub fn delete_quota(user_id: &String, context: &UserContext) -> Result<(), enums::DbError> {
    if context.is_admin != true.to_string() {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::quotas::dsl::*;
    match diesel::update(quotas.filter(id.eq(user_id)))
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

/// Permanently deletes a quota entry from the database.
///
/// This function removes a quota entry from the database completely.
/// Only admin users can access this function.
///
/// # Arguments
/// * `user_id` - The ID of the user to permanently delete the quota for
/// * `context` - The user context containing authentication information
pub fn hard_delete_quota(user_id: &String, context: &UserContext) {
    if context.is_admin != true.to_string() {
        return;
    }

    use self::quotas::dsl::*;
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    let _ = diesel::delete(quotas.filter(id.eq(user_id.to_string()))).execute(&mut *conn);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn hard_delete_quota(user_id: &String) {
        use self::quotas::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(quotas.filter(id.eq(user_id))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_quota() {
        let _ = init_quota_table();
        let project_id = "test-project-1".to_string();
        let owner_id = "test-quota-1".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let quota = QuotaEntry {
            id: owner_id.clone(),
            max_model: 42,
            max_dataset: 43,
            max_checkpoint: 44,
            max_secret: 45,
            max_taskqueue: 46,
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_quota(&quota.id);

        add_quota(&quota).unwrap();
        if let Ok(retrieved_quota) = get_quota(&owner_id, &context) {
            assert_eq!(retrieved_quota.id, quota.id);
            assert_eq!(retrieved_quota.max_model, quota.max_model);
            assert_eq!(retrieved_quota.max_dataset, quota.max_dataset);
            assert_eq!(retrieved_quota.max_checkpoint, quota.max_checkpoint);
            assert_eq!(retrieved_quota.max_secret, quota.max_secret);
            assert_eq!(retrieved_quota.max_taskqueue, quota.max_taskqueue);
            assert_eq!(retrieved_quota.status, quota.status);
            assert_eq!(retrieved_quota.created_by, quota.created_by);
            assert_eq!(retrieved_quota.updated_by, quota.updated_by);
            assert_eq!(retrieved_quota.deleted_at, quota.deleted_at);
            assert_eq!(retrieved_quota.deleted_by, quota.deleted_by);
        };

        let _ = delete_quota(&quota.id, &context);
    }

    #[test]
    #[serial]
    fn test_set_quota() {
        let _ = init_quota_table();
        let project_id = "test-project-1".to_string();
        let owner_id = "test-quota-1".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let quota = QuotaEntry {
            id: owner_id.clone(),
            max_model: 42,
            max_dataset: 43,
            max_checkpoint: 44,
            max_secret: 45,
            max_taskqueue: 46,
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_quota(&quota.id);

        add_quota(&quota).unwrap();

        let new_max_model = 52;
        let new_max_dataset = 53;
        let new_max_checkpoint = 54;
        let new_max_secret = 55;
        let new_max_taskqueue = 56;

        // set new quota
        assert!(
            set_quota(
                &owner_id,
                new_max_model,
                new_max_dataset,
                new_max_checkpoint,
                new_max_secret,
                new_max_taskqueue,
                &context
            )
            .is_ok()
        );

        if let Ok(retrieved_quota) = get_quota(&owner_id, &context) {
            assert_eq!(retrieved_quota.id, quota.id);
            assert_eq!(retrieved_quota.max_model, new_max_model);
            assert_eq!(retrieved_quota.max_dataset, new_max_dataset);
            assert_eq!(retrieved_quota.max_checkpoint, new_max_checkpoint);
            assert_eq!(retrieved_quota.max_secret, new_max_secret);
            assert_eq!(retrieved_quota.max_taskqueue, new_max_taskqueue);
            assert_eq!(retrieved_quota.status, quota.status);
            assert_eq!(retrieved_quota.created_by, quota.created_by);
            assert_eq!(retrieved_quota.updated_by, quota.updated_by);
            assert_eq!(retrieved_quota.deleted_at, quota.deleted_at);
            assert_eq!(retrieved_quota.deleted_by, quota.deleted_by);
        };

        let _ = delete_quota(&quota.id, &context);
    }

    #[test]
    #[serial]
    fn test_list_quotas() {
        let _ = init_quota_table();
        let project_id = "test-project-1".to_string();
        let owner_id1 = "test-quota-2".to_string();
        let owner_id2 = "test-quota-3".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id1.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let user1 = QuotaEntry {
            id: owner_id1.clone(),
            max_model: 42,
            max_dataset: 43,
            max_checkpoint: 44,
            max_secret: 45,
            max_taskqueue: 46,
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let user2 = QuotaEntry {
            id: owner_id2.clone(),
            max_model: 42,
            max_dataset: 43,
            max_checkpoint: 44,
            max_secret: 45,
            max_taskqueue: 46,
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_quota(&user1.id);
        hard_delete_quota(&user2.id);

        add_quota(&user1).unwrap();
        add_quota(&user2).unwrap();

        let quotas = list_quotas(&context).unwrap();
        assert_eq!(quotas.len(), 1);

        let _ = delete_quota(&user1.id, &context);
        let _ = delete_quota(&user2.id, &context);
    }

    #[test]
    #[serial]
    fn test_delete_quota() {
        let _ = init_quota_table();
        let project_id = "test-project-1".to_string();
        let owner_id = "test-quota-4".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let quota = QuotaEntry {
            id: owner_id.clone(),
            max_model: 42,
            max_dataset: 43,
            max_checkpoint: 44,
            max_secret: 45,
            max_taskqueue: 46,
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_quota(&quota.id);

        add_quota(&quota).unwrap();
        let _ = delete_quota(&owner_id, &context);
        let result = get_quota(&owner_id, &context);
        assert!(result.is_err());
    }
}
