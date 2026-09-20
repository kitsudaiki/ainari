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

// Define the schema for networks table
table! {
    networks (uuid) {
        uuid -> Varchar,
        name -> Varchar,
        subnet -> Varchar,
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

/// Represents an entry in the networks table.
/// This struct contains all the fields required to create, query, and update meta network records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = networks)]
pub struct NetworkEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub name: String,
    pub subnet: String,
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

/// Initializes the networks table in the database if it doesn't exist.
///
/// This function creates the table with the appropriate schema and constraints.
/// It's typically called during application startup to ensure the required tables exist.
pub fn init_network_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS networks (
        uuid VARCHAR(40) PRIMARY KEY,
        name VARCHAR(256),
        subnet VARCHAR(40),
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

/// Adds a new meta network to the database.
///
/// This function creates a new NetworkEntry with the provided parameters and inserts it into the database.
/// The status is set to "ACTIVE" and timestamps are set to the current time.
///
/// # Arguments
/// * `network_uuid` - The unique identifier for the meta network
/// * `network_name` - The name of the meta network
/// * `subnet` - The subnet of the network in CIDR notation
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_network(
    network_uuid: &Uuid,
    network_name: &str,
    subnet: &str,
    context: &UserContext,
) -> QueryResult<usize> {
    let network = NetworkEntry {
        uuid: *network_uuid,
        name: network_name.to_string().clone(),
        subnet: subnet.to_string(),
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

    add_network(network)
}

/// Adds a meta network to the database.
///
/// This is a helper function that performs the actual insertion of a NetworkEntry into the database.
///
/// # Arguments
/// * `network` - The NetworkEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_network(network: NetworkEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    diesel::insert_into(networks)
        .values(network)
        .execute(&mut *conn)
}

/// Retrieves a meta network from the database.
///
/// This function queries the database for a meta network with the specified UUID and checks the user's permissions.
/// Only active networks are returned, and the query is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `network_uuid` - The UUID of the meta network to retrieve
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result containing the NetworkEntry if found, or a DbError if not found or an error occurs
pub fn get_network(
    network_uuid: &Uuid,
    context: &UserContext,
) -> Result<NetworkEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    let mut query = networks
        .filter(uuid.eq(network_uuid.to_string()).and(status.eq("ACTIVE")))
        .into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    match query
        .select(NetworkEntry::as_select())
        .first::<NetworkEntry>(&mut *conn)
    {
        Ok(network) => Ok(network),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all meta networks that the user has access to.
///
/// This function retrieves all active meta networks and applies permission-based filtering.
/// The results are filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing a vector of NetworkEntry objects
#[allow(dead_code)]
pub fn list_networks(context: &UserContext) -> QueryResult<Vec<NetworkEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    let mut query = networks.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(NetworkEntry::as_select()).load(&mut *conn)
}

/// Counts the number of meta networks that the user has access to.
///
/// This function counts all active meta networks and applies permission-based filtering.
/// The count is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing the count of meta networks as an i64
pub fn count_networks(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;

    let mut query = networks.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Force deletes a meta network from the database.
///
/// This function marks a meta network as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Arguments
/// * `network_uuid` - The UUID of the meta network to delete
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn force_delete_network(network_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    match diesel::update(networks.filter(uuid.eq(network_uuid.to_string())))
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

/// Deletes a meta network from the database.
///
/// This function marks a meta network as deleted after verifying that the user has permission to delete it.
/// It first checks if the network exists and if the user has the necessary permissions.
///
/// # Arguments
/// * `network_uuid` - The UUID of the meta network to delete
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_network(network_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // Verify the meta network exists and the user has permission to delete it
    get_network(network_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    match diesel::update(networks.filter(uuid.eq(network_uuid.to_string())))
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

/// Deletes all meta networks from the database.
///
/// This function marks all active meta networks as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_all_network() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::networks::dsl::*;
    match diesel::update(networks.filter(status.eq("ACTIVE")))
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

    fn hard_delete_network(network_uuid: &Uuid) {
        use self::networks::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(networks.filter(uuid.eq(network_uuid.to_string()))).execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_add_get_network() {
        let _ = init_network_table();
        let uuid1 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let network = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_network(&uuid1);

        add_network(network.clone()).unwrap();
        match get_network(&uuid1, &context) {
            Ok(retrieved_network) => {
                assert_eq!(retrieved_network.uuid, network.uuid);
                assert_eq!(retrieved_network.subnet, network.subnet);
                assert_eq!(retrieved_network.owner_id, network.owner_id);
                assert_eq!(retrieved_network.project_id, network.project_id);
                assert_eq!(retrieved_network.status, network.status);
                assert_eq!(retrieved_network.created_by, network.created_by);
                assert_eq!(retrieved_network.updated_by, network.updated_by);
                assert_eq!(retrieved_network.deleted_at, network.deleted_at);
                assert_eq!(retrieved_network.deleted_by, network.deleted_by);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_network(&uuid1);
    }

    #[test]
    #[serial]
    fn test_list_networks() {
        let _ = init_network_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let network1 = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let network2 = NetworkEntry {
            uuid: uuid2,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "DELETED".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);

        add_network(network1).unwrap();
        add_network(network2).unwrap();
        let networks = list_networks(&context).unwrap();
        assert_eq!(networks.len(), 1);
        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_network() {
        let _ = init_network_table();
        let uuid1 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let network = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_network(&uuid1);

        add_network(network.clone()).unwrap();
        let _ = delete_network(&uuid1, &context);
        let result = get_network(&uuid1, &context);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_count_networks() {
        let _ = init_network_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let project_id = "test-project".to_string();
        let owner_id = "test-user".to_string();
        let context = UserContext {
            token: "".to_string(),
            user_id: owner_id.clone(),
            project_id: project_id.clone(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        let network1 = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let network2 = NetworkEntry {
            uuid: uuid2,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let network3 = NetworkEntry {
            uuid: uuid3,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: owner_id.clone(),
            project_id: project_id.clone(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);

        add_network(network1).unwrap();
        add_network(network2).unwrap();
        add_network(network3).unwrap();

        let number = count_networks(&context).unwrap();
        assert_eq!(number, 3);

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);
    }

    #[test]
    #[serial]
    fn test_networks_permissions() {
        let _ = init_network_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let name = "test-network".to_string();
        let subnet = "127.0.0.1".to_string();

        let network1 = NetworkEntry {
            uuid: uuid1,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let network2 = NetworkEntry {
            uuid: uuid2,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: "test-user-43".to_string(),
            project_id: "test_permissions_1".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        let network3 = NetworkEntry {
            uuid: uuid3,
            name: name.clone(),
            subnet: subnet.clone(),
            owner_id: "test-user-44".to_string(),
            project_id: "test_permissions_2".to_string(),
            status: "ACTIVE".to_string(),
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
            deleted_at: None,
            deleted_by: None,
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);

        add_network(network1).unwrap();
        add_network(network2).unwrap();
        add_network(network3).unwrap();

        // list-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        let networks = list_networks(&context).unwrap();
        assert_eq!(networks.len(), 1);

        // list-test project-admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: true.to_string(),
        };
        let networks = list_networks(&context).unwrap();
        assert_eq!(networks.len(), 2);

        // list-test admin
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: true.to_string(),
            is_project_admin: false.to_string(),
        };
        let networks = list_networks(&context).unwrap();
        assert_eq!(networks.len(), 3);

        // get-test normal user
        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user-42".to_string(),
            project_id: "test_permissions_1".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };
        match get_network(&uuid1, &context) {
            Ok(retrieved_network) => {
                assert_eq!(retrieved_network.uuid, uuid1);
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
        if get_network(&uuid3, &context).is_ok() {
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
        if delete_network(&uuid3, &context).is_ok() {
            assert_eq!(true, false);
        };

        hard_delete_network(&uuid1);
        hard_delete_network(&uuid2);
        hard_delete_network(&uuid3);
    }
}
