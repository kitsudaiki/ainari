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
use diesel::dsl::count_star;
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

// Define the schema
table! {
    snapshots (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        onsen_address -> Varchar,
        file_path -> Text,
        secret_uuid -> Varchar,
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

#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = snapshots)]
pub struct SnapshotEntry {
    pub uuid: String,
    pub name: String,
    pub onsen_address: String,
    pub file_path: String,
    pub secret_uuid: String,
    pub owner_id: String,
    pub project_id: String,
    pub status: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

/// Creates the snapshot-table, if it does not already exist.
///
/// # Returns
/// * `Result<(), Box<dyn Error>>` - Ok, if the table is available, else the database-error
pub fn init_snapshot_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS snapshots (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        onsen_address VARCHAR(256),
        file_path TEXT,
        secret_uuid VARCHAR(40),
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

/// Builds a new snapshot-entry from the given values and inserts it into the database.
///
/// The ownership-fields and the timestamps are filled from the user-context, so all entries are
/// created in the same way.
///
/// # Arguments
/// * `snapshot_uuid` - The UUID of the new snapshot
/// * `snapshot_name` - The name of the new snapshot
/// * `onsen_address` - The address of the onsen, where the payload is stored
/// * `file_path` - The path of the payload on the onsen
/// * `secret_uuid` - The UUID of the secret, which was used to encrypt the payload
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
pub fn add_new_snapshot(
    snapshot_uuid: &Uuid,
    snapshot_name: &str,
    onsen_address: &str,
    file_path: &str,
    secret_uuid: &Uuid,
    context: &UserContext,
) -> QueryResult<usize> {
    let snapshot = SnapshotEntry {
        uuid: snapshot_uuid.to_string().clone(),
        name: snapshot_name.to_owned(),
        onsen_address: onsen_address.to_owned(),
        file_path: file_path.to_owned(),
        secret_uuid: secret_uuid.to_string().clone(),
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now().to_rfc3339(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_snapshot(&snapshot)
}

/// Inserts an already built snapshot-entry into the database.
///
/// # Arguments
/// * `snapshot` - The entry to insert
///
/// # Returns
/// * `QueryResult<usize>` - The number of rows affected by the insert operation
pub fn add_snapshot(snapshot: &SnapshotEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::snapshots::dsl::*;

    diesel::insert_into(snapshots)
        .values(snapshot)
        .execute(&mut *conn)
}

/// Retrieves a snapshot from the database.
///
/// Only active snapshots are returned, so an already deleted one is reported as not found. An
/// admin sees every snapshot, a project-admin all snapshots of his project and every other
/// user only his own ones.
///
/// # Arguments
/// * `snapshot_uuid` - The UUID of the snapshot to retrieve
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<SnapshotEntry, enums::DbError>` - The requested snapshot or an error
pub fn get_snapshot(
    snapshot_uuid: &Uuid,
    context: &UserContext,
) -> Result<SnapshotEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::snapshots::dsl::*;

    let mut query = snapshots
        .filter(uuid.eq(snapshot_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(SnapshotEntry::as_select())
        .first::<SnapshotEntry>(&mut *conn)
    {
        Ok(snapshot) => Ok(snapshot),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all snapshots, which are visible for the user.
///
/// Uses the same visibility-rules as `get_snapshot`.
///
/// # Arguments
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<Vec<SnapshotEntry>>` - All visible snapshots or a database-error
pub fn list_snapshots(context: &UserContext) -> QueryResult<Vec<SnapshotEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::snapshots::dsl::*;

    let mut query = snapshots.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(SnapshotEntry::as_select()).load(&mut *conn)
}

/// Counts the active snapshots of the requesting user.
///
/// In contrast to `list_snapshots` this always counts only the own snapshots of the user, also
/// for an admin, because the result is used to check the quota of that user.
///
/// # Arguments
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `QueryResult<i64>` - The number of snapshots or a database-error
pub fn count_snapshots(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::snapshots::dsl::*;

    let mut query = snapshots.filter(status.eq("ACTIVE")).into_boxed();

    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Deletes a snapshot from the database.
///
/// The entry is not removed, but only marked as deleted together with the timestamp and the user,
/// who deleted it, so the history stays available. The snapshot is read first, so a user can
/// only delete a snapshot, which he is allowed to see.
///
/// # Arguments
/// * `snapshot_uuid` - The UUID of the snapshot to delete
/// * `context` - The user context containing authentication information
///
/// # Returns
/// * `Result<(), enums::DbError>` - Ok, if the snapshot was marked as deleted, else an error
pub fn delete_snapshot(snapshot_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    get_snapshot(snapshot_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::snapshots::dsl::*;

    match diesel::update(snapshots.filter(uuid.eq(snapshot_uuid.to_string())))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn hard_delete_snapshot(snapshot_uuid: &Uuid) {
        use self::snapshots::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(snapshots.filter(uuid.eq(snapshot_uuid.to_string())))
            .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_snapshot() {
        let _ = init_snapshot_table();
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let snapshot = SnapshotEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_snapshot(&uuid1);

        add_snapshot(&snapshot).unwrap();
        if let Ok(retrieved_snapshot) = get_snapshot(&uuid1, &context) {
            assert_eq!(retrieved_snapshot.uuid, snapshot.uuid);
            assert_eq!(retrieved_snapshot.name, snapshot.name);
            assert_eq!(retrieved_snapshot.file_path, snapshot.file_path);
            assert_eq!(retrieved_snapshot.status, snapshot.status);
            assert_eq!(retrieved_snapshot.created_by, snapshot.created_by);
            assert_eq!(retrieved_snapshot.updated_by, snapshot.updated_by);
            assert_eq!(retrieved_snapshot.deleted_at, snapshot.deleted_at);
            assert_eq!(retrieved_snapshot.deleted_by, snapshot.deleted_by);
        };

        hard_delete_snapshot(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_snapshots() {
        let _ = init_snapshot_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let snapshot1 = SnapshotEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let snapshot2 = SnapshotEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_snapshot(&uuid1);
        hard_delete_snapshot(&uuid2);

        add_snapshot(&snapshot1).unwrap();
        add_snapshot(&snapshot2).unwrap();
        let snapshots = list_snapshots(&context).unwrap();
        assert_eq!(snapshots.len(), 1);
        hard_delete_snapshot(&uuid1);
        hard_delete_snapshot(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_snapshot() {
        let _ = init_snapshot_table();
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let snapshot = SnapshotEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_snapshot(&uuid1);

        add_snapshot(&snapshot).unwrap();
        let _ = delete_snapshot(&uuid1, &context);
        let result = get_snapshot(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_snapshots() {
        let _ = init_snapshot_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-snapshot".to_string();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let snapshot1 = SnapshotEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let snapshot2 = SnapshotEntry {
            uuid: uuid2.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let snapshot3 = SnapshotEntry {
            uuid: uuid3.to_string(),
            name: name.clone(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_snapshot(&uuid1);
        hard_delete_snapshot(&uuid2);
        hard_delete_snapshot(&uuid3);

        add_snapshot(&snapshot1).unwrap();
        add_snapshot(&snapshot2).unwrap();
        add_snapshot(&snapshot3).unwrap();

        let number = count_snapshots(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_snapshot(&uuid1);
        hard_delete_snapshot(&uuid2);
        hard_delete_snapshot(&uuid3);
    }

    #[test]
    #[serial]
    fn test_snapshots_permissions() {
        let _ = init_snapshot_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let snapshot1 = SnapshotEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let snapshot2 = SnapshotEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let snapshot3 = SnapshotEntry {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            onsen_address: onsen_address.clone(),
            file_path: "/tmp/bla".to_string(),
            secret_uuid: secret_uuid.to_string().clone(),
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_snapshot(&uuid1);
        hard_delete_snapshot(&uuid2);
        hard_delete_snapshot(&uuid3);

        add_snapshot(&snapshot1).unwrap();
        add_snapshot(&snapshot2).unwrap();
        add_snapshot(&snapshot3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let snapshots = list_snapshots(&context).unwrap();
        assert_eq!(snapshots.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let snapshots = list_snapshots(&context).unwrap();
        assert_eq!(snapshots.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let snapshots = list_snapshots(&context).unwrap();
        assert_eq!(snapshots.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_snapshot(&uuid1, &context) {
            Ok(retrieved_snapshot) => {
                assert_eq!(retrieved_snapshot.uuid, uuid1.to_string());
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
        if get_snapshot(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        // delete-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        if delete_snapshot(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_snapshot(&uuid1);
        hard_delete_snapshot(&uuid2);
        hard_delete_snapshot(&uuid3);
    }
}
