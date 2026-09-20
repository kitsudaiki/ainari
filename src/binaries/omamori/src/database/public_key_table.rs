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
use diesel::connection::SimpleConnection;
use diesel::dsl::count_star;
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for public_keys table
table! {
    public_keys (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        public_key -> Varchar,
        fingerprint -> Varchar,
        owner_id -> Varchar,
        project_id -> Varchar,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents an entry in the public_keys table.
/// This struct contains all the fields required to create, query, and update public-key records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = public_keys)]
pub struct PublicKeyEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    pub public_key: String,
    pub fingerprint: String,
    pub owner_id: String,
    pub project_id: String,
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

/// Initializes the public_keys table in the database if it doesn't exist.
///
/// This function creates the table with the appropriate schema and constraints.
/// It's typically called during application startup to ensure the required tables exist.
pub fn init_public_key_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS public_keys (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        public_key TEXT,
        fingerprint VARCHAR(256),
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
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

/// Adds a new public-key to the database.
///
/// This function creates a new PublicKeyEntry with the provided parameters and inserts it into the database.
/// The status is set to "ACTIVE" and timestamps are set to the current time.
///
/// # Arguments
/// * `public_key_uuid` - The unique identifier for the public-key
/// * `name` - The human-readable name for the public-key
/// * `public_key` - The public-key itself
/// * `fingerprint` - The fingerprint of the public-key
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_public_key(
    public_key_uuid: &Uuid,
    name: &str,
    public_key: &str,
    fingerprint: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    let new_public_key = PublicKeyEntry {
        uuid: *public_key_uuid,
        name: name.to_string(),
        public_key: public_key.to_string(),
        fingerprint: fingerprint.to_string(),
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_public_key(new_public_key)
}

/// Adds a public-key to the database.
///
/// This is a helper function that performs the actual insertion of a PublicKeyEntry into the database.
///
/// # Arguments
/// * `entry` - The PublicKeyEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_public_key(entry: PublicKeyEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::public_keys::dsl::*;
    diesel::insert_into(public_keys)
        .values(entry)
        .execute(&mut *conn)
}

/// Retrieves a public-key from the database.
///
/// This function queries the database for a public-key with the specified UUID and checks the user's permissions.
/// Only active public_keys are returned, and the query is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `public_key_uuid` - The UUID of the public-key to retrieve
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result containing the PublicKeyEntry if found, or a DbError if not found or an error occurs
pub fn get_public_key(
    public_key_uuid: &Uuid,
    context: &UserContext,
) -> Result<PublicKeyEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::public_keys::dsl::*;

    let mut query = public_keys
        .filter(
            uuid.eq(public_key_uuid.to_string())
                .and(status.eq("ACTIVE")),
        )
        .into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(PublicKeyEntry::as_select())
        .first::<PublicKeyEntry>(&mut *conn)
    {
        Ok(entry) => Ok(entry),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all public-keys that the user has access to.
///
/// This function retrieves all active public-keys and applies permission-based filtering.
/// The results are filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing a vector of PublicKeyEntry objects
#[allow(dead_code)]
pub fn list_public_keys(context: &UserContext) -> QueryResult<Vec<PublicKeyEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::public_keys::dsl::*;

    let mut query = public_keys.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(PublicKeyEntry::as_select()).load(&mut *conn)
}

/// Counts the number of public-keys that the user has access to.
///
/// This function counts all active public-keys and applies permission-based filtering.
/// The count is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing the count of public-keys as an i64
#[allow(dead_code)]
pub fn count_public_keys(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::public_keys::dsl::*;

    let mut query = public_keys.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Force deletes a public-key from the database.
///
/// This function marks a public-key as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Arguments
/// * `public_key_uuid` - The UUID of the public-key to delete
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn force_delete_public_key(public_key_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::public_keys::dsl::*;
    match diesel::update(public_keys.filter(uuid.eq(public_key_uuid.to_string())))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("HOST_INIT"),
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

/// Deletes a public-key from the database.
///
/// This function marks a public-key as deleted after verifying that the user has permission to delete it.
/// It first checks if the public-key exists and if the user has the necessary permissions.
///
/// # Arguments
/// * `public_key_uuid` - The UUID of the public-key to delete
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_public_key(
    public_key_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // Verify the public-key exists and the user has permission to delete it
    get_public_key(public_key_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::public_keys::dsl::*;
    match diesel::update(public_keys.filter(uuid.eq(public_key_uuid.to_string())))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq(context.user_id.clone()),
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

/// Deletes all public-keys from the database.
///
/// This function marks all active public-keys as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_all_public_key() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::public_keys::dsl::*;
    match diesel::update(public_keys.filter(status.eq("ACTIVE")))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("AINARI_START"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    const PUBLIC_KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITestKeyForUnitTests test-key";
    const FINGERPRINT: &str = "SHA256:tYq0AoG5KGT1p8wLk3zXn4Rd2vB7mCsEiQxHfJuNaOw";

    fn hard_delete_public_key(public_key_uuid: &Uuid) {
        use self::public_keys::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(public_keys.filter(uuid.eq(public_key_uuid.to_string())))
            .execute(&mut *conn);
    }

    /// Builds a PublicKeyEntry for the tests with the given identity and status.
    fn new_entry(
        entry_uuid: &Uuid,
        entry_owner_id: &str,
        entry_project_id: &str,
        entry_status: &str,
    ) -> PublicKeyEntry {
        PublicKeyEntry {
            uuid: *entry_uuid,
            name: "test-public-key".to_string(),
            public_key: PUBLIC_KEY.to_string(),
            fingerprint: FINGERPRINT.to_string(),
            owner_id: entry_owner_id.to_string(),
            project_id: entry_project_id.to_string(),
            status: entry_status.to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        }
    }

    /// Builds a UserContext for the tests.
    fn new_context(
        user_id: &str,
        project_id: &str,
        is_admin: bool,
        is_project_admin: bool,
    ) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: user_id.to_string(),
            project_id: project_id.to_string(),
            is_admin: is_admin.to_string(),
            is_project_admin: is_project_admin.to_string(),
        }
    }

    /// Unwraps a get-result. `enums::DbError` implements neither `Debug` nor `PartialEq`,
    /// so the results can not be handled by `expect` and `assert_eq`.
    fn expect_entry(result: Result<PublicKeyEntry, enums::DbError>) -> PublicKeyEntry {
        match result {
            Ok(entry) => entry,
            Err(_) => panic!("public-key was not found"),
        }
    }

    /// Asserts that a get-result reports a missing entry.
    fn assert_not_found(result: Result<PublicKeyEntry, enums::DbError>) {
        assert!(matches!(result, Err(enums::DbError::NotFound)));
    }

    #[test]
    #[serial]
    fn test_add_get_public_key() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry = new_entry(&uuid1, "test-user", "test-project", "ACTIVE");

        hard_delete_public_key(&uuid1);

        add_public_key(entry.clone()).unwrap();
        let retrieved = expect_entry(get_public_key(&uuid1, &context));

        assert_eq!(retrieved.uuid, entry.uuid);
        assert_eq!(retrieved.name, entry.name);
        assert_eq!(retrieved.public_key, entry.public_key);
        assert_eq!(retrieved.fingerprint, entry.fingerprint);
        assert_eq!(retrieved.owner_id, entry.owner_id);
        assert_eq!(retrieved.project_id, entry.project_id);
        assert_eq!(retrieved.status, entry.status);
        assert_eq!(retrieved.created_at, entry.created_at);
        assert_eq!(retrieved.created_by, entry.created_by);
        assert_eq!(retrieved.updated_at, entry.updated_at);
        assert_eq!(retrieved.updated_by, entry.updated_by);
        assert_eq!(retrieved.deleted_at, entry.deleted_at);
        assert_eq!(retrieved.deleted_by, entry.deleted_by);

        hard_delete_public_key(&uuid1);
    }

    #[test]
    #[serial]
    fn test_get_public_key_not_found() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        hard_delete_public_key(&uuid1);

        assert_not_found(get_public_key(&uuid1, &context));
    }

    #[test]
    #[serial]
    fn test_list_public_keys() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry1 = new_entry(&uuid1, "test-user", "test-project", "ACTIVE");
        let entry2 = new_entry(&uuid2, "test-user", "test-project", "DELETED");

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);

        add_public_key(entry1).unwrap();
        add_public_key(entry2).unwrap();

        // only the ACTIVE entry is listed
        let entries = list_public_keys(&context).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, uuid1);
        assert_eq!(entries[0].public_key, PUBLIC_KEY);
        assert_eq!(entries[0].fingerprint, FINGERPRINT);

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_public_key() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry = new_entry(&uuid1, "test-user", "test-project", "ACTIVE");

        hard_delete_public_key(&uuid1);

        add_public_key(entry).unwrap();
        assert!(delete_public_key(&uuid1, &context).is_ok());

        assert_not_found(get_public_key(&uuid1, &context));

        hard_delete_public_key(&uuid1);
    }

    #[test]
    #[serial]
    fn test_force_delete_public_key() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // the entry belongs to another project, so only the force-delete can remove it
        let entry = new_entry(&uuid1, "other-user", "other-project", "ACTIVE");

        hard_delete_public_key(&uuid1);

        add_public_key(entry).unwrap();
        assert!(force_delete_public_key(&uuid1).is_ok());

        assert_not_found(get_public_key(&uuid1, &context));

        hard_delete_public_key(&uuid1);
    }

    #[test]
    #[serial]
    fn test_delete_all_public_key() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", true, false);

        let entry1 = new_entry(&uuid1, "test-user", "test-project", "ACTIVE");
        let entry2 = new_entry(&uuid2, "other-user", "other-project", "ACTIVE");

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);

        add_public_key(entry1).unwrap();
        add_public_key(entry2).unwrap();

        assert!(delete_all_public_key().is_ok());

        assert_eq!(list_public_keys(&context).unwrap().len(), 0);

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);
    }

    #[test]
    #[serial]
    fn test_count_public_keys() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry1 = new_entry(&uuid1, "test-user", "test-project", "ACTIVE");
        let entry2 = new_entry(&uuid2, "test-user", "test-project", "ACTIVE");
        // neither DELETED entries nor entries of other owners are counted
        let entry3 = new_entry(&uuid3, "other-user", "test-project", "ACTIVE");

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);
        hard_delete_public_key(&uuid3);

        add_public_key(entry1).unwrap();
        add_public_key(entry2).unwrap();
        add_public_key(entry3).unwrap();

        assert_eq!(count_public_keys(&context).unwrap(), 2);

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);
        hard_delete_public_key(&uuid3);
    }

    #[test]
    #[serial]
    fn test_public_keys_permissions() {
        let _ = init_public_key_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        let entry1 = new_entry(&uuid1, "test-user-42", "test_permissions_1", "ACTIVE");
        let entry2 = new_entry(&uuid2, "test-user-43", "test_permissions_1", "ACTIVE");
        let entry3 = new_entry(&uuid3, "test-user-44", "test_permissions_2", "ACTIVE");

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);
        hard_delete_public_key(&uuid3);

        add_public_key(entry1).unwrap();
        add_public_key(entry2).unwrap();
        add_public_key(entry3).unwrap();

        // list-test normal user
        let context = new_context("test-user-42", "test_permissions_1", false, false);
        assert_eq!(list_public_keys(&context).unwrap().len(), 1);

        // list-test project-admin
        let context = new_context("test-user-42", "test_permissions_1", false, true);
        assert_eq!(list_public_keys(&context).unwrap().len(), 2);

        // list-test admin
        let context = new_context("test-user-42", "test_permissions_1", true, false);
        assert_eq!(list_public_keys(&context).unwrap().len(), 3);

        // get-test normal user
        let context = new_context("test-user-42", "test_permissions_1", false, false);
        let retrieved = expect_entry(get_public_key(&uuid1, &context));
        assert_eq!(retrieved.uuid, uuid1);

        // get-test normal user, entry of another user within the same project
        assert!(get_public_key(&uuid2, &context).is_err());

        // get-test normal user, entry of another project
        assert!(get_public_key(&uuid3, &context).is_err());

        // get-test project-admin, entry of another user within the same project
        let context = new_context("test-user-42", "test_permissions_1", false, true);
        let retrieved = expect_entry(get_public_key(&uuid2, &context));
        assert_eq!(retrieved.uuid, uuid2);

        // get-test admin, entry of another project
        let context = new_context("test-user-42", "test_permissions_1", true, false);
        let retrieved = expect_entry(get_public_key(&uuid3, &context));
        assert_eq!(retrieved.uuid, uuid3);

        // delete-test normal user, entry of another project
        let context = new_context("test-user-42", "test_permissions_1", false, false);
        assert!(delete_public_key(&uuid3, &context).is_err());

        hard_delete_public_key(&uuid1);
        hard_delete_public_key(&uuid2);
        hard_delete_public_key(&uuid3);
    }
}
