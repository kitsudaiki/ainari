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
    secrets (uuid) {
        uuid -> Varchar,
        name -> Varchar,
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
#[diesel(table_name = secrets)]
pub struct SecretEntry {
    pub uuid: String,
    pub name: String,
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

pub fn init_secret_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS secrets (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
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

pub fn add_new_secret(secret_uuid: &Uuid, name: &str, context: &UserContext) -> QueryResult<usize> {
    let secret = SecretEntry {
        uuid: secret_uuid.to_string().clone(),
        name: name.to_owned(),
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

    add_secret(&secret)
}

pub fn add_secret(secret: &SecretEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;
    diesel::insert_into(secrets)
        .values(secret)
        .execute(&mut *conn)
}

pub fn get_secret(
    secret_uuid: &Uuid,
    context: &UserContext,
) -> Result<SecretEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    let mut query = secrets
        .filter(uuid.eq(secret_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(SecretEntry::as_select())
        .first::<SecretEntry>(&mut *conn)
    {
        Ok(secret) => Ok(secret),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

#[allow(dead_code)]
pub fn list_secrets(context: &UserContext) -> QueryResult<Vec<SecretEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    let mut query = secrets.filter(status.eq("ACTIVE")).into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(SecretEntry::as_select()).load(&mut *conn)
}

pub fn count_secrets(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    let mut query = secrets.filter(status.eq("ACTIVE")).into_boxed();

    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

pub fn delete_secret(secret_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    get_secret(secret_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;
    match diesel::update(secrets.filter(uuid.eq(secret_uuid.to_string())))
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

#[allow(dead_code)]
pub fn delete_all_secret() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;
    match diesel::update(secrets.filter(status.eq("ACTIVE")))
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

    fn hard_delete_secret(secret_uuid: &Uuid) {
        use self::secrets::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(secrets.filter(uuid.eq(secret_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_secret() {
        let _ = init_secret_table();
        let uuid1 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let secret = SecretEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
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

        hard_delete_secret(&uuid1);

        add_secret(&secret).unwrap();
        match get_secret(&uuid1, &context) {
            Ok(retrieved_secret) => {
                assert_eq!(retrieved_secret.uuid, secret.uuid);
                assert_eq!(retrieved_secret.name, secret.name);
                assert_eq!(retrieved_secret.owner_id, secret.owner_id);
                assert_eq!(retrieved_secret.project_id, secret.project_id);
                assert_eq!(retrieved_secret.status, secret.status);
                assert_eq!(retrieved_secret.created_by, secret.created_by);
                assert_eq!(retrieved_secret.updated_by, secret.updated_by);
                assert_eq!(retrieved_secret.deleted_at, secret.deleted_at);
                assert_eq!(retrieved_secret.deleted_by, secret.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_secret(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_secrets() {
        let _ = init_secret_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let secret1 = SecretEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
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

        let secret2 = SecretEntry {
            uuid: uuid2.to_string(),
            name: name.clone(),
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

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);

        add_secret(&secret1).unwrap();
        add_secret(&secret2).unwrap();
        let secrets = list_secrets(&context).unwrap();
        assert_eq!(secrets.len(), 1);
        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_secret() {
        let _ = init_secret_table();
        let uuid1 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let secret = SecretEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
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

        hard_delete_secret(&uuid1);

        add_secret(&secret).unwrap();
        let _ = delete_secret(&uuid1, &context);
        let result = get_secret(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_secrets() {
        let _ = init_secret_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let secret1 = SecretEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
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

        let secret2 = SecretEntry {
            uuid: uuid2.to_string(),
            name: name.clone(),
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

        let secret3 = SecretEntry {
            uuid: uuid3.to_string(),
            name: name.clone(),
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

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);

        add_secret(&secret1).unwrap();
        add_secret(&secret2).unwrap();
        add_secret(&secret3).unwrap();

        let number = count_secrets(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);
    }

    #[test]
    #[serial]
    fn test_secrets_permissions() {
        let _ = init_secret_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let secret1 = SecretEntry {
            uuid: uuid1.to_string(),
            name: name.clone(),
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

        let secret2 = SecretEntry {
            uuid: uuid2.to_string(),
            name: name.clone(),
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

        let secret3 = SecretEntry {
            uuid: uuid3.to_string(),
            name: name.clone(),
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

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);

        add_secret(&secret1).unwrap();
        add_secret(&secret2).unwrap();
        add_secret(&secret3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let secrets = list_secrets(&context).unwrap();
        assert_eq!(secrets.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let secrets = list_secrets(&context).unwrap();
        assert_eq!(secrets.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let secrets = list_secrets(&context).unwrap();
        assert_eq!(secrets.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_secret(&uuid1, &context) {
            Ok(retrieved_secret) => {
                assert_eq!(retrieved_secret.uuid, uuid1.to_string());
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
        if get_secret(&uuid3, &context).is_ok() {
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
        if delete_secret(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);
    }
}
