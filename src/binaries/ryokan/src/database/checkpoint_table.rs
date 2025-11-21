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
use diesel::dsl::count_star;
use diesel::prelude::*;
use std::error::Error;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

// Define the schema
table! {
    checkpoints (uuid) {
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
#[diesel(table_name = checkpoints)]
pub struct CheckpointEntry {
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

pub fn init_checkpoint_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS checkpoints (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        onsen_address VARCHAR(256),
        file_path TEXT,
        secret_uuid VARCHAR(32),
        owner_id VARCHAR(256),
        project_id VARCHAR(256),
        status VARCHAR(10),
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

pub fn add_new_checkpoint(
    checkpoint_uuid: &Uuid,
    checkpoint_name: &str,
    onsen_address: &str,
    file_path: &str,
    secret_uuid: &Uuid,
    context: &UserContext,
) -> QueryResult<usize> {
    let checkpoint = CheckpointEntry {
        uuid: checkpoint_uuid.to_string().clone(),
        name: checkpoint_name.to_owned(),
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

    add_checkpoint(&checkpoint)
}

pub fn add_checkpoint(checkpoint: &CheckpointEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::checkpoints::dsl::*;

    diesel::insert_into(checkpoints)
        .values(checkpoint)
        .execute(&mut *conn)
}

pub fn get_checkpoint(
    checkpoint_uuid: &Uuid,
    context: &UserContext,
) -> Result<CheckpointEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::checkpoints::dsl::*;

    let mut query = checkpoints
        .filter(
            uuid.eq(checkpoint_uuid.to_string())
                .and(status.eq("ACTIVE")),
        )
        .into_boxed();

    if !context.is_admin {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if !context.is_project_admin {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(CheckpointEntry::as_select())
        .first::<CheckpointEntry>(&mut *conn)
    {
        Ok(checkpoint) => Ok(checkpoint),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_checkpoints(context: &UserContext) -> QueryResult<Vec<CheckpointEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::checkpoints::dsl::*;

    let mut query = checkpoints.filter(status.eq("ACTIVE")).into_boxed();

    if !context.is_admin {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if !context.is_project_admin {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(CheckpointEntry::as_select()).load(&mut *conn)
}

pub fn count_checkpoints(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::checkpoints::dsl::*;

    let mut query = checkpoints.filter(status.eq("ACTIVE")).into_boxed();

    if !context.is_admin {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if !context.is_project_admin {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(count_star()).first::<i64>(&mut *conn)
}

pub fn delete_checkpoint(
    checkpoint_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    get_checkpoint(checkpoint_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::checkpoints::dsl::*;

    match diesel::update(checkpoints.filter(uuid.eq(checkpoint_uuid.to_string())))
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

    fn hard_delete_checkpoint(checkpoint_uuid: &Uuid) {
        use self::checkpoints::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(checkpoints.filter(uuid.eq(checkpoint_uuid.to_string())))
            .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_checkpoint() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let checkpoint = CheckpointEntry {
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

        hard_delete_checkpoint(&uuid1);

        add_checkpoint(&checkpoint).unwrap();
        if let Ok(retrieved_checkpoint) = get_checkpoint(&uuid1, &context) {
            assert_eq!(retrieved_checkpoint.uuid, checkpoint.uuid);
            assert_eq!(retrieved_checkpoint.name, checkpoint.name);
            assert_eq!(retrieved_checkpoint.file_path, checkpoint.file_path);
            assert_eq!(retrieved_checkpoint.status, checkpoint.status);
            assert_eq!(retrieved_checkpoint.created_by, checkpoint.created_by);
            assert_eq!(retrieved_checkpoint.updated_by, checkpoint.updated_by);
            assert_eq!(retrieved_checkpoint.deleted_at, checkpoint.deleted_at);
            assert_eq!(retrieved_checkpoint.deleted_by, checkpoint.deleted_by);
        };

        hard_delete_checkpoint(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_checkpoints() {
        let _ = init_checkpoint_table();
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
            is_admin: false,
            is_project_admin: false,
        };

        let checkpoint1 = CheckpointEntry {
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

        let checkpoint2 = CheckpointEntry {
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

        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);

        add_checkpoint(&checkpoint1).unwrap();
        add_checkpoint(&checkpoint2).unwrap();
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 1);
        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_checkpoint() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let checkpoint = CheckpointEntry {
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

        hard_delete_checkpoint(&uuid1);

        add_checkpoint(&checkpoint).unwrap();
        let _ = delete_checkpoint(&uuid1, &context);
        let result = get_checkpoint(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_checkpoints() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-checkpoint".to_string();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false,
            is_project_admin: false,
        };

        let checkpoint1 = CheckpointEntry {
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

        let checkpoint2 = CheckpointEntry {
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

        let checkpoint3 = CheckpointEntry {
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

        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);
        hard_delete_checkpoint(&uuid3);

        add_checkpoint(&checkpoint1).unwrap();
        add_checkpoint(&checkpoint2).unwrap();
        add_checkpoint(&checkpoint3).unwrap();

        let number = count_checkpoints(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);
        hard_delete_checkpoint(&uuid3);
    }

    #[test]
    #[serial]
    fn test_checkpoints_permissions() {
        let _ = init_checkpoint_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let onsen_address = "127.0.0.1:1234".to_string();
        let secret_uuid = Uuid::new_v4();

        let checkpoint1 = CheckpointEntry {
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

        let checkpoint2 = CheckpointEntry {
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

        let checkpoint3 = CheckpointEntry {
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

        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);
        hard_delete_checkpoint(&uuid3);

        add_checkpoint(&checkpoint1).unwrap();
        add_checkpoint(&checkpoint2).unwrap();
        add_checkpoint(&checkpoint3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: true,
        };
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true,
            is_project_admin: false,
        };
        let checkpoints = list_checkpoints(&context).unwrap();
        assert_eq!(checkpoints.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        match get_checkpoint(&uuid1, &context) {
            Ok(retrieved_checkpoint) => {
                assert_eq!(retrieved_checkpoint.uuid, uuid1.to_string());
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
            is_admin: false,
            is_project_admin: false,
        };
        if get_checkpoint(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        // delete-test normal user false uuid
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false,
            is_project_admin: false,
        };
        if delete_checkpoint(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_checkpoint(&uuid1);
        hard_delete_checkpoint(&uuid2);
        hard_delete_checkpoint(&uuid3);
    }
}
