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

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;

// Define the schema
table! {
    hosts (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        address -> Varchar,
        status -> Varchar,
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
        deleted_at -> Nullable<Varchar>,
        deleted_by -> Nullable<Varchar>,
    }
}

/// Represents a host entry in the database.
///
/// This struct maps to the `hosts` table in the database and contains all the fields
/// necessary to create, read, update, and delete host records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = hosts)]
pub struct HostEntry {
    /// Unique identifier for the host
    pub uuid: String,
    /// Human-readable name for the host
    pub name: String,
    /// Network address of the host
    pub address: String,
    /// Current status of the host (ACTIVE, DELETED, etc.)
    pub status: String,
    /// Timestamp when the host was created
    pub created_at: String,
    /// User ID who created the host
    pub created_by: String,
    /// Timestamp when the host was last updated
    pub updated_at: String,
    /// User ID who last updated the host
    pub updated_by: String,
    /// Timestamp when the host was deleted (if applicable)
    pub deleted_at: Option<String>,
    /// User ID who deleted the host (if applicable)
    pub deleted_by: Option<String>,
}

/// Initializes the hosts table in the database if it doesn't already exist.
///
/// This function creates the table with all necessary columns and constraints.
/// It should be called during application startup to ensure the table exists.
///
/// # Returns
/// * `Ok(())` if the table was successfully created or already exists
/// * An error if there was an issue executing the SQL command
pub fn init_host_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS hosts (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        address VARCHAR(256),
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

/// Adds a new host to the database with default values.
///
/// This function creates a new HostEntry with the provided UUID, name, and address,
/// sets the status to "ACTIVE", and uses the current timestamp and user context
/// for creation and update information.
///
/// # Arguments
/// * `host_uuid` - Unique identifier for the new host
/// * `host_name` - Human-readable name for the host
/// * `host_address` - Network address of the host
/// * `context` - User context containing information about the user performing the action
///
/// # Returns
/// * QueryResult containing the number of rows affected
pub fn add_new_host(
    host_uuid: &Uuid,
    host_name: &str,
    host_address: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    let host = HostEntry {
        uuid: host_uuid.to_string().clone(),
        name: host_name.to_owned(),
        address: host_address.to_owned(),
        status: "ACTIVE".to_string(),
        created_at: Utc::now().to_rfc3339(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now().to_rfc3339(),
        updated_by: context.user_id.clone(),
        deleted_at: None,
        deleted_by: None,
    };

    add_host(&host)
}

/// Adds a host to the database.
///
/// This is a lower-level function that takes a fully constructed HostEntry and
/// inserts it into the database.
///
/// # Arguments
/// * `host` - HostEntry to be inserted into the database
///
/// # Returns
/// * QueryResult containing the number of rows affected
pub fn add_host(host: &HostEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    diesel::insert_into(hosts).values(host).execute(&mut *conn)
}

/// Retrieves a host by its network address.
///
/// This function queries the database for a host with the given address that
/// has an "ACTIVE" status.
///
/// # Arguments
/// * `host_address` - Network address of the host to retrieve
/// * `context` - User context (not currently used in the query)
///
/// # Returns
/// * Ok(HostEntry) if a matching host is found
/// * DbError::NotFound if no matching host is found
/// * DbError::InternalError if there was an error executing the query
pub fn get_host_by_address(
    host_address: &String,
    _: &UserContext,
) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts
        .filter(name.eq(host_address.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    match query
        .select(HostEntry::as_select())
        .first::<HostEntry>(&mut *conn)
    {
        Ok(host) => Ok(host),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Retrieves a host by its UUID.
///
/// This function queries the database for a host with the given UUID that
/// has an "ACTIVE" status.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to retrieve
/// * `context` - User context (not currently used in the query)
///
/// # Returns
/// * Ok(HostEntry) if a matching host is found
/// * DbError::NotFound if no matching host is found
/// * DbError::InternalError if there was an error executing the query
pub fn get_host(host_uuid: &Uuid, _: &UserContext) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts
        .filter(uuid.eq(host_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    match query
        .select(HostEntry::as_select())
        .first::<HostEntry>(&mut *conn)
    {
        Ok(host) => Ok(host),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all active hosts in the database.
///
/// This function retrieves all hosts that have an "ACTIVE" status.
///
/// # Arguments
/// * `context` - User context (not currently used in the query)
///
/// # Returns
/// * QueryResult containing a vector of HostEntry objects
pub fn list_hosts(_: &UserContext) -> QueryResult<Vec<HostEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let query = hosts.filter(status.eq("ACTIVE")).into_boxed();

    query.select(HostEntry::as_select()).load(&mut *conn)
}

/// Deletes a host by marking it as "DELETED".
///
/// This function updates the host's status to "DELETED" and sets the deletion
/// timestamp and user. It first verifies that the host exists and is active.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to delete
/// * `context` - User context containing information about the user performing the deletion
///
/// # Returns
/// * Ok(()) if the host was successfully deleted
/// * DbError::NotFound if the host doesn't exist or is already deleted
/// * DbError::InternalError if there was an error executing the query
pub fn delete_host_admin(host_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    get_host(host_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::update(hosts.filter(uuid.eq(host_uuid.to_string())))
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

/// Deletes all active hosts by marking them as "DELETED".
///
/// This function updates all hosts with an "ACTIVE" status to "DELETED"
/// and sets the deletion timestamp and user. This is typically used for
/// database reset or cleanup purposes.
///
/// # Returns
/// * Ok(()) if all hosts were successfully deleted
/// * DbError::NotFound if no active hosts were found
/// * DbError::InternalError if there was an error executing the query
#[allow(dead_code)]
pub fn delete_all_host() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::update(hosts.filter(status.eq("ACTIVE")))
        .set((
            status.eq("DELETED"),
            deleted_at.eq(Utc::now().to_rfc3339()),
            deleted_by.eq("HANAMI_START"),
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

    fn hard_delete_host(host_uuid: &Uuid) {
        use self::hosts::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(hosts.filter(uuid.eq(host_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_host() {
        let _ = init_host_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let host = HostEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_host(&uuid1);

        add_host(&host).unwrap();
        match get_host(&uuid1, &context) {
            Ok(retrieved_host) => {
                assert_eq!(retrieved_host.uuid, host.uuid);
                assert_eq!(retrieved_host.name, host.name);
                assert_eq!(retrieved_host.status, host.status);
                assert_eq!(retrieved_host.created_by, host.created_by);
                assert_eq!(retrieved_host.updated_by, host.updated_by);
                assert_eq!(retrieved_host.deleted_at, host.deleted_at);
                assert_eq!(retrieved_host.deleted_by, host.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_host(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_hosts() {
        let _ = init_host_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let host1 = HostEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let host2 = HostEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            status: "DELETED".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);

        add_host(&host1).unwrap();
        add_host(&host2).unwrap();
        let hosts = list_hosts(&context).unwrap();
        assert_eq!(hosts.len(), 1);
        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_host() {
        let _ = init_host_table();
        let uuid1 = Uuid::new_v4();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let host = HostEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_host(&uuid1);

        add_host(&host).unwrap();
        let _ = delete_host_admin(&uuid1, &context);
        let result = get_host(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_hosts_permissions() {
        let _ = init_host_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();

        let host1 = HostEntry {
            uuid: uuid1.to_string(),
            name: "Alice".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let host2 = HostEntry {
            uuid: uuid2.to_string(),
            name: "Bob".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let host3 = HostEntry {
            uuid: uuid3.to_string(),
            name: "Poi".to_string(),
            address: "http://127.0.0.1:11420".to_string(),
            status: "ACTIVE".to_string(),
            created_at: "2025-03-31".to_string(),
            created_by: "admin".to_string(),
            updated_at: "2025-03-31".to_string(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);
        hard_delete_host(&uuid3);

        add_host(&host1).unwrap();
        add_host(&host2).unwrap();
        add_host(&host3).unwrap();

        // list-test
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let hosts = list_hosts(&context).unwrap();
        assert_eq!(hosts.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_host(&uuid1, &context) {
            Ok(retrieved_host) => {
                assert_eq!(retrieved_host.uuid, uuid1.to_string());
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_host(&uuid1);
        hard_delete_host(&uuid2);
        hard_delete_host(&uuid3);
    }
}
