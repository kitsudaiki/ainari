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
use rand::{Rng, distr::Alphanumeric};
use std::env;
use std::error::Error;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::functions::sha256_hash;
use ainari_common::secret::Secret;

// Define the schema
table! {
    users (id) {
        id -> Varchar,
        name -> Varchar,
        projects -> Text,
        is_admin -> Varchar,
        pw_hash -> Varchar,
        salt -> Varchar,
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
#[diesel(table_name = users)]
pub struct UserEntry {
    pub id: String,
    pub name: String,
    pub projects: String,
    pub is_admin: String,
    pub pw_hash: String,
    pub salt: String,
    pub status: String,
    pub created_at: String,
    pub created_by: String,
    pub updated_at: String,
    pub updated_by: String,
    pub deleted_at: Option<String>,
    pub deleted_by: Option<String>,
}

pub fn init_admin() -> Result<(), Box<dyn Error>> {
    let fake_admin_context = UserContext {
        token: "".to_string(),
        user_id: "AINARI_INIT".to_string(),
        project_id: "AINARI_INIT".to_string(),
        is_admin: true.to_string(),
        is_project_admin: false.to_string(),
    };

    let users = list_users(&fake_admin_context).unwrap();
    if !users.is_empty() {
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

    let admin_name: String = match env::var("AINARI_ADMIN_NAME") {
        Ok(val) => val,
        Err(_) => {
            log::error!("couldn't find env-variable: AINARI_ADMIN_NAME");
            return Err("An error occurred while initializing new admin-user".into());
        }
    };

    let admin_passphrase: Secret = match env::var("AINARI_ADMIN_PASSPHRASE") {
        Ok(val) => Secret::from(val),
        Err(_) => {
            log::error!("couldn't find env-variable: AINARI_ADMIN_PASSPHRASE");
            return Err("An error occurred while initializing new admin-user".into());
        }
    };

    add_new_user(
        &admin_id,
        &admin_name,
        &admin_passphrase,
        &true.to_string(),
        &fake_admin_context,
    )?;

    Ok(())
}

pub fn init_user_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS users (
        id VARCHAR(256),
        name VARCHAR(256),
        is_admin VARCHAR(8),
        pw_hash VARCHAR(64),
        salt VARCHAR(64),
        projects TEXT,
        status VARCHAR(8),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(256)
    );",
    )?;
    // release lock on the connection to avoid dead-lock
    drop(conn);

    init_admin()
}

pub fn add_new_user(
    user_id: &String,
    user_name: &str,
    passphrase: &Secret,
    is_admin: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    if context.is_admin != true.to_string() {
        return Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            Box::new("Permission denied.".to_string()),
        ));
    }

    // check if user alredy exist in the database
    // The same id is allowed multiple times in the table, but only one time active.
    if get_user(user_id, context).is_ok() {
        return Err(diesel::result::Error::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new(format!("User with ID '{user_id}' already exist.")),
        ));
    };

    // salt passphrase
    let salt: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    let salted_passphrase = Secret::from(format!("{}{salt}", passphrase.reveal()));

    // create sha256-hash from the salted passphrase to store the hash in the database
    let pw_hash = sha256_hash(salted_passphrase.reveal());

    let user = UserEntry {
        id: user_id.clone(),
        name: user_name.to_owned(),
        projects: "[]".to_string(),
        is_admin: is_admin.to_string(),
        pw_hash,
        salt,
        status: "ACTIVE".to_string(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now().to_rfc3339(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_user(&user)
}

pub fn add_user(user: &UserEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::users::dsl::*;

    diesel::insert_into(users).values(user).execute(&mut *conn)
}

pub fn get_auth_user(user_id: &String) -> Result<UserEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::users::dsl::*;
    match users
        .filter(id.eq(user_id).and(status.eq("ACTIVE")))
        .select(UserEntry::as_select())
        .first::<UserEntry>(&mut *conn)
    {
        Ok(user) => Ok(user),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn get_user(user_id: &String, context: &UserContext) -> Result<UserEntry, enums::DbError> {
    if context.is_admin != true.to_string() {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::users::dsl::*;
    match users
        .filter(id.eq(user_id).and(status.eq("ACTIVE")))
        .select(UserEntry::as_select())
        .first::<UserEntry>(&mut *conn)
    {
        Ok(user) => Ok(user),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

pub fn list_users(context: &UserContext) -> QueryResult<Vec<UserEntry>> {
    if context.is_admin != true.to_string() {
        let dummy: QueryResult<Vec<UserEntry>> = Ok(vec![]);
        return dummy;
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::users::dsl::*;
    users
        .filter(status.eq("ACTIVE"))
        .select(UserEntry::as_select())
        .load(&mut *conn)
}

pub fn delete_user(user_id: &String, context: &UserContext) -> Result<(), enums::DbError> {
    if context.is_admin != true.to_string() {
        return Err(enums::DbError::NotFound);
    }

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::users::dsl::*;
    match diesel::update(users.filter(id.eq(user_id)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn hard_delete_user(user_id: &String) {
        use self::users::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(users.filter(id.eq(user_id))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_user() {
        let _ = init_user_table();
        let project_id = "test-project-1".to_string();
        let owner_id = "test-user-1".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let user = UserEntry {
            id: owner_id.clone(),
            name: "Alice".to_string(),
            projects: "ProjectA".to_string(),
            is_admin: true.to_string(),
            pw_hash: "hash123".to_string(),
            salt: "salt123".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_user(&user.id);

        add_user(&user).unwrap();
        if let Ok(retrieved_user) = get_user(&owner_id, &context) {
            assert_eq!(retrieved_user.id, user.id);
            assert_eq!(retrieved_user.name, user.name);
            assert_eq!(retrieved_user.projects, user.projects);
            assert_eq!(retrieved_user.is_admin, user.is_admin);
            assert_eq!(retrieved_user.pw_hash, user.pw_hash);
            assert_eq!(retrieved_user.salt, user.salt);
            assert_eq!(retrieved_user.status, user.status);
            assert_eq!(retrieved_user.created_by, user.created_by);
            assert_eq!(retrieved_user.updated_by, user.updated_by);
            assert_eq!(retrieved_user.deleted_at, user.deleted_at);
            assert_eq!(retrieved_user.deleted_by, user.deleted_by);
        };

        let _ = delete_user(&user.id, &context);
    }

    #[test]
    #[serial]
    fn test_list_users() {
        let _ = init_user_table();
        let project_id = "test-project-1".to_string();
        let owner_id1 = "test-user-2".to_string();
        let owner_id2 = "test-user-3".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id1.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let user1 = UserEntry {
            id: owner_id1.clone(),
            name: "Alice".to_string(),
            projects: "ProjectA".to_string(),
            is_admin: true.to_string(),
            pw_hash: "hash123".to_string(),
            salt: "salt123".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let user2 = UserEntry {
            id: owner_id2.clone(),
            name: "Bob".to_string(),
            projects: "ProjectB".to_string(),
            is_admin: false.to_string(),
            pw_hash: "hash456".to_string(),
            salt: "salt456".to_string(),
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_user(&user1.id);
        hard_delete_user(&user2.id);

        add_user(&user1).unwrap();
        add_user(&user2).unwrap();

        let users = list_users(&context).unwrap();
        assert_eq!(users.len(), 1);

        let _ = delete_user(&user1.id, &context);
        let _ = delete_user(&user2.id, &context);
    }

    #[test]
    #[serial]
    fn test_delete_user() {
        let _ = init_user_table();
        let project_id = "test-project-1".to_string();
        let owner_id = "test-user-4".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };

        let user = UserEntry {
            id: owner_id.clone(),
            name: "Alice".to_string(),
            projects: "ProjectA".to_string(),
            is_admin: true.to_string(),
            pw_hash: "hash123".to_string(),
            salt: "salt123".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_user(&user.id);

        add_user(&user).unwrap();
        let _ = delete_user(&owner_id, &context);
        let result = get_user(&owner_id, &context);
        assert!(result.is_err());
    }
}
