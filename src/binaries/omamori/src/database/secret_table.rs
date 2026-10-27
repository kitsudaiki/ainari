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
use diesel::dsl::count_star;
use diesel::prelude::*;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::secret_structs::SECRET_OWNER_BACKUP;
use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for the secrets table
table! {
    secrets (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        owned_by -> Varchar,
        resource_uuid -> Nullable<Varchar>,
        owner_id -> Varchar,
        project_id -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
    }
}

/// Represents a secret entry in the database
///
/// This struct maps to the `secrets` table in the database and contains all fields
/// necessary to create, read, update, and delete secret records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = secrets)]
pub struct SecretEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    pub owned_by: String,
    #[diesel(serialize_as = DbOptUuid, deserialize_as = DbOptUuid)]
    pub resource_uuid: Option<Uuid>,
    pub owner_id: String,
    pub project_id: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

/// Adds a new secret to the database with default values
///
/// This function creates a new SecretEntry with the provided UUID and name,
/// using the current timestamp and user context for metadata fields.
///
/// # Arguments
///
/// * `secret_uuid` - The unique identifier for the new secret
/// * `name` - The human-readable name for the secret
/// * `owned_by` - The kind of owner of the secret, for example `user`
/// * `resource_uuid` - The UUID of the resource, which owns the secret, if there is one
/// * `context` - The user context containing information about the current user
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected by the insert operation
pub fn add_new_secret(
    secret_uuid: &Uuid,
    name: &str,
    owned_by: &str,
    resource_uuid: Option<Uuid>,
    context: &UserContext,
) -> QueryResult<usize> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

    let secret = SecretEntry {
        uuid: *secret_uuid,
        name: name.to_owned(),
        owned_by: owned_by.to_owned(),
        resource_uuid,
        owner_id: context.user_id.clone(),
        project_id: context.project_id.clone(),
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
    };

    add_secret(secret.clone())
}

/// Adds a secret to the database
///
/// This function inserts the provided SecretEntry into the database.
///
/// # Arguments
///
/// * `secret` - The SecretEntry to insert into the database
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected by the insert operation
pub fn add_secret(secret: SecretEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;
    diesel::insert_into(secrets)
        .values(secret)
        .execute(&mut *conn)
}

/// Retrieves a secret from the database
///
/// This function fetches a secret by its UUID, applying appropriate filters
/// based on the user's permissions.
///
/// # Arguments
///
/// * `secret_uuid` - The UUID of the secret to retrieve
/// * `context` - The user context containing information about the current user
///
/// # Returns
///
/// A Result containing the SecretEntry if found, or a DbError if not found
/// or if an internal error occurs
pub fn get_secret(
    secret_uuid: &Uuid,
    context: &UserContext,
) -> Result<SecretEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    let mut query = secrets
        .filter(uuid.eq(secret_uuid.to_string()))
        .into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
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

/// Lists all secrets that the user has access to
///
/// This function returns all secrets that are visible to the user
/// based on their permissions. Admins can see all secrets, all other users can see all secrets
/// of their project, also the ones of other users.
///
/// # Arguments
///
/// * `context` - The user context containing information about the current user
///
/// # Returns
///
/// A QueryResult containing a vector of SecretEntry objects
pub fn list_secrets(context: &UserContext) -> QueryResult<Vec<SecretEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    let mut query = secrets.into_boxed();

    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
    }

    query.select(SecretEntry::as_select()).load(&mut *conn)
}

/// Counts the number of secrets that the user owns
///
/// This function returns the count of secrets owned by the current user
/// within their project. Only accessible to non-admin users.
///
/// # Arguments
///
/// * `context` - The user context containing information about the current user
///
/// # Returns
///
/// A QueryResult containing the count of secrets as an i64
pub fn count_secrets(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    let mut query = secrets.into_boxed();

    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Counts the number of secrets of a whole project.
///
/// Unlike `count_secrets`, the secrets of all users of the project are counted, because the quota,
/// which is checked with this number, belongs to the project. The secrets of backups are not
/// counted, because they belong to the backups, which can outlive the project, so they neither
/// count to the quota nor block the deletion of the project.
///
/// # Arguments
///
/// * `project` - The ID of the project, which is counted
///
/// # Returns
///
/// A QueryResult containing the count of secrets as an i64
pub fn count_secrets_of_project(project: &str) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    secrets
        .filter(project_id.eq(project))
        .filter(owned_by.ne(SECRET_OWNER_BACKUP))
        .select(count_star())
        .first::<i64>(&mut *conn)
}

/// Lists all secrets of a project.
///
/// Unlike the listing for the users, it is done for the project of the path and not the one of
/// the token, so an admin can export any project.
///
/// # Arguments
///
/// * `project` - The ID of the project
///
/// # Returns
///
/// A QueryResult containing the secrets of the project
pub fn list_secrets_of_project(project: &str) -> QueryResult<Vec<SecretEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    secrets
        .filter(project_id.eq(project))
        .select(SecretEntry::as_select())
        .load(&mut *conn)
}

/// Searches a secret by its UUID within all projects.
///
/// It is used to check, if the UUID is still free, before a secret with this UUID is imported.
///
/// # Arguments
///
/// * `entry_uuid` - The UUID of the secret
///
/// # Returns
///
/// A QueryResult containing the secret, or None, if the UUID is not used
pub fn find_secret(entry_uuid: &Uuid) -> QueryResult<Option<SecretEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    secrets
        .filter(uuid.eq(entry_uuid.to_string()))
        .select(SecretEntry::as_select())
        .first::<SecretEntry>(&mut *conn)
        .optional()
}

/// Adds a secret to the database or replaces the existing entry with the same UUID.
///
/// It is used to import a secret with its original UUID, which is the primary key of the table.
///
/// # Arguments
///
/// * `entry` - The secret to store
///
/// # Returns
///
/// A QueryResult indicating the number of rows affected by the insert operation
pub fn put_secret(entry: SecretEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;

    conn.transaction(|conn| {
        diesel::delete(secrets.filter(uuid.eq(entry.uuid.to_string()))).execute(conn)?;
        diesel::insert_into(secrets).values(entry).execute(conn)
    })
}

/// Deletes a secret from the database
///
/// This function removes a secret from the database.
///
/// # Arguments
///
/// * `secret_uuid` - The UUID of the secret to delete
/// * `context` - The user context containing information about the current user
///
/// # Returns
///
/// A Result indicating success or failure
pub fn delete_secret(secret_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    get_secret(secret_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;
    match diesel::delete(secrets.filter(uuid.eq(secret_uuid.to_string()))).execute(&mut *conn) {
        Ok(_) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Deletes all secrets in the database
///
/// This function deletes all secrets. This is typically
/// used for cleanup during application startup or shutdown.
///
/// # Returns
///
/// A Result indicating success or failure
#[allow(dead_code)]
pub fn delete_all_secret() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::secrets::dsl::*;
    match diesel::delete(secrets).execute(&mut *conn) {
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
    use ainari_common::enums::ProjectRole;
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
        let uuid1 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let secret = SecretEntry {
            uuid: uuid1,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_secret(&uuid1);

        add_secret(secret.clone()).unwrap();
        match get_secret(&uuid1, &context) {
            Ok(retrieved_secret) => {
                assert_eq!(retrieved_secret.uuid, secret.uuid);
                assert_eq!(retrieved_secret.name, secret.name);
                assert_eq!(retrieved_secret.owned_by, secret.owned_by);
                assert_eq!(retrieved_secret.resource_uuid, secret.resource_uuid);
                assert_eq!(retrieved_secret.owner_id, secret.owner_id);
                assert_eq!(retrieved_secret.project_id, secret.project_id);
                assert_eq!(retrieved_secret.created_by, secret.created_by);
                assert_eq!(retrieved_secret.updated_by, secret.updated_by);
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
            project_role: ProjectRole::Member.to_string(),
        };

        let secret1 = SecretEntry {
            uuid: uuid1,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let secret2 = SecretEntry {
            uuid: uuid2,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);

        add_secret(secret1).unwrap();
        add_secret(secret2).unwrap();
        assert!(delete_secret(&uuid2, &context).is_ok());
        let secrets = list_secrets(&context).unwrap();
        assert_eq!(secrets.len(), 1);
        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_secret() {
        let uuid1 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };

        let secret = SecretEntry {
            uuid: uuid1,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_secret(&uuid1);

        add_secret(secret).unwrap();
        let _ = delete_secret(&uuid1, &context);
        let result = get_secret(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_secrets() {
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
            project_role: ProjectRole::Member.to_string(),
        };

        let secret1 = SecretEntry {
            uuid: uuid1,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let secret2 = SecretEntry {
            uuid: uuid2,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let secret3 = SecretEntry {
            uuid: uuid3,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);

        add_secret(secret1).unwrap();
        add_secret(secret2).unwrap();
        add_secret(secret3).unwrap();

        let number = count_secrets(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);
    }

    #[test]
    #[serial]
    fn test_secrets_permissions() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-secret".to_string();

        let secret1 = SecretEntry {
            uuid: uuid1,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let secret2 = SecretEntry {
            uuid: uuid2,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        let secret3 = SecretEntry {
            uuid: uuid3,
            name: name.clone(),
            owned_by: "user".to_string(),
            resource_uuid: None,
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        };

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);

        add_secret(secret1).unwrap();
        add_secret(secret2).unwrap();
        add_secret(secret3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let secrets = list_secrets(&context).unwrap();
        // members see all entries of their project, also the ones of other users
        assert_eq!(secrets.len(), 2);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Admin.to_string(),
        };
        let secrets = list_secrets(&context).unwrap();
        assert_eq!(secrets.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let secrets = list_secrets(&context).unwrap();
        assert_eq!(secrets.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        match get_secret(&uuid1, &context) {
            Ok(retrieved_secret) => {
                assert_eq!(retrieved_secret.uuid, uuid1);
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
            project_role: ProjectRole::Member.to_string(),
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
            project_role: ProjectRole::Member.to_string(),
        };
        if delete_secret(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_secret(&uuid1);
        hard_delete_secret(&uuid2);
        hard_delete_secret(&uuid3);
    }

    #[test]
    #[serial]
    fn test_backup_secrets_are_not_counted() {
        let project = "test-backup-secret-project";
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: project.to_string(),
            is_admin: false.to_string(),
            project_role: ProjectRole::Member.to_string(),
        };
        let user_secret = Uuid::new_v4();
        let image_secret = Uuid::new_v4();
        let backup_secret = Uuid::new_v4();
        add_new_secret(&user_secret, "user-secret", "user", None, &context).unwrap();
        add_new_secret(
            &image_secret,
            "image-secret",
            "image",
            Some(Uuid::new_v4()),
            &context,
        )
        .unwrap();
        add_new_secret(
            &backup_secret,
            "backup-secret",
            SECRET_OWNER_BACKUP,
            Some(Uuid::new_v4()),
            &context,
        )
        .unwrap();

        // the secret of the backup neither counts to the quota nor blocks the project-deletion
        assert_eq!(count_secrets_of_project(project).unwrap(), 2);
        assert!(get_secret(&backup_secret, &context).is_ok());

        hard_delete_secret(&user_secret);
        hard_delete_secret(&image_secret);
        hard_delete_secret(&backup_secret);
    }
}
