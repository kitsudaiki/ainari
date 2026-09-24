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
use rand::prelude::IndexedRandom;
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
        number_of_cores -> BigInt,
        used_number_of_cores -> BigInt,
        memory_size -> BigInt,
        amount_of_used_memory -> BigInt,
        disk_space -> BigInt,
        amount_of_used_disk_space -> BigInt,
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
    /// Number of cpu-threads of the host
    pub number_of_cores: i64,
    /// Number of cpu-threads, which are already in use
    pub used_number_of_cores: i64,
    /// Total memory of the host in MiB
    pub memory_size: i64,
    /// Amount of memory in MiB, which is already in use
    pub amount_of_used_memory: i64,
    /// Total size of the disk for the virtual-machines in GiB
    pub disk_space: i64,
    /// Amount of disk-space in GiB, which is already in use
    pub amount_of_used_disk_space: i64,
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

/// Hardware-resources of a host, which are reported by the host itself at registration.
#[derive(Debug, PartialEq, Clone)]
pub struct HostResources {
    /// Number of cpu-threads of the host
    pub number_of_cores: i64,
    /// Total memory of the host in MiB
    pub memory_size: i64,
    /// Total size of the disk for the virtual-machines in GiB
    pub disk_space: i64,
}

/// Resource-columns of the hosts table. All of them are non-negative integers.
const RESOURCE_COLUMNS: [&str; 6] = [
    "number_of_cores",
    "used_number_of_cores",
    "memory_size",
    "amount_of_used_memory",
    "disk_space",
    "amount_of_used_disk_space",
];

/// Initializes the hosts table in the database if it doesn't already exist.
///
/// This function creates the table with all necessary columns and constraints and adds
/// the resource-columns, if they are missing in an already existing table.
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
        number_of_cores BIGINT NOT NULL DEFAULT 0 CHECK (number_of_cores >= 0),
        used_number_of_cores BIGINT NOT NULL DEFAULT 0 CHECK (used_number_of_cores >= 0),
        memory_size BIGINT NOT NULL DEFAULT 0 CHECK (memory_size >= 0),
        amount_of_used_memory BIGINT NOT NULL DEFAULT 0 CHECK (amount_of_used_memory >= 0),
        disk_space BIGINT NOT NULL DEFAULT 0 CHECK (disk_space >= 0),
        amount_of_used_disk_space BIGINT NOT NULL DEFAULT 0 CHECK (amount_of_used_disk_space >= 0),
        status VARCHAR(8),
        created_at VARCHAR(64),
        created_by VARCHAR(256),
        updated_at VARCHAR(64),
        updated_by VARCHAR(256),
        deleted_at VARCHAR(64),
        deleted_by VARCHAR(256)
    );",
    )?;

    // tables of older versions were created without the resource-columns, so they are added
    // there at the end of the table, because sqlite can not insert columns in between
    for column in RESOURCE_COLUMNS {
        let sql = format!(
            "ALTER TABLE hosts ADD COLUMN {column} BIGINT NOT NULL DEFAULT 0 CHECK ({column} >= 0);"
        );
        match conn.batch_execute(&sql) {
            Ok(()) => {}
            Err(e) if e.to_string().contains("duplicate column name") => {}
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

/// Adds a new host to the database with default values.
///
/// This function creates a new HostEntry with the provided UUID, name, address and
/// hardware-resources, sets the status to "ACTIVE", sets the used resources to 0 and uses
/// the current timestamp and user context for creation and update information.
///
/// # Arguments
/// * `host_uuid` - Unique identifier for the new host
/// * `host_name` - Human-readable name for the host
/// * `host_address` - Network address of the host
/// * `resources` - Hardware-resources of the host
/// * `context` - User context containing information about the user performing the action
///
/// # Returns
/// * QueryResult containing the number of rows affected
pub fn add_new_host(
    host_uuid: &Uuid,
    host_name: &str,
    host_address: &str,
    resources: &HostResources,
    context: &UserContext,
) -> QueryResult<usize> {
    let host = HostEntry {
        uuid: host_uuid.to_string().clone(),
        name: host_name.to_owned(),
        address: host_address.to_owned(),
        number_of_cores: resources.number_of_cores,
        used_number_of_cores: 0,
        memory_size: resources.memory_size,
        amount_of_used_memory: 0,
        disk_space: resources.disk_space,
        amount_of_used_disk_space: 0,
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

/// Updates the hardware-resources of an existing host.
///
/// This is used, when an already registered host registers itself again, because
/// its hardware could have changed in the meantime.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host to update
/// * `resources` - New hardware-resources of the host
/// * `context` - User context containing information about the user performing the action
///
/// # Returns
/// * Ok(()) if the host was successfully updated
/// * DbError::NotFound if the host doesn't exist or is not active
/// * DbError::InternalError if there was an error executing the query
pub fn update_host_resources(
    host_uuid: &Uuid,
    resources: &HostResources,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    match diesel::update(hosts.filter(uuid.eq(host_uuid.to_string()).and(status.eq("ACTIVE"))))
        .set((
            number_of_cores.eq(resources.number_of_cores),
            memory_size.eq(resources.memory_size),
            disk_space.eq(resources.disk_space),
            updated_at.eq(Utc::now().to_rfc3339()),
            updated_by.eq(context.user_id.clone()),
        ))
        .execute(&mut *conn)
    {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
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
/// * `_` - User context (not currently used in the query)
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
        .filter(
            address
                .eq(host_address.to_string())
                .and(status.eq("ACTIVE")),
        )
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
/// * `_` - User context (not currently used in the query)
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
/// * `_` - User context (not currently used in the query)
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

/// Selects a random active host, which has enough free resources for the requested values,
/// and allocates the requested resources on this host.
///
/// A host is suitable, if for cores, memory and disk the total value minus the used value
/// is greater or equal to the requested value. The used values of the selected host are
/// increased by the requested values.
///
/// The check and the allocation run while the database-connection is locked and within one
/// immediate transaction, so no other request can allocate the same resources in between.
///
/// # Arguments
/// * `requested` - Resources, which are requested by the new virtual-machine
/// * `_` - User context (not currently used in the query)
///
/// # Returns
/// * Ok(HostEntry) with the selected host and its updated used values
/// * DbError::NotFound if no host has enough free resources
/// * DbError::InternalError if there was an error executing the queries
pub fn allocate_host_resources(
    requested: &HostResources,
    _: &UserContext,
) -> Result<HostEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;

    let result = conn.immediate_transaction::<_, diesel::result::Error, _>(|conn| {
        let suitable_hosts = hosts
            .filter(status.eq("ACTIVE"))
            .filter((number_of_cores - used_number_of_cores).ge(requested.number_of_cores))
            .filter((memory_size - amount_of_used_memory).ge(requested.memory_size))
            .filter((disk_space - amount_of_used_disk_space).ge(requested.disk_space))
            .select(HostEntry::as_select())
            .load::<HostEntry>(conn)?;

        let selected_host = suitable_hosts
            .choose(&mut rand::rng())
            .ok_or(diesel::result::Error::NotFound)?;

        diesel::update(hosts.filter(uuid.eq(&selected_host.uuid)))
            .set((
                used_number_of_cores.eq(used_number_of_cores + requested.number_of_cores),
                amount_of_used_memory.eq(amount_of_used_memory + requested.memory_size),
                amount_of_used_disk_space.eq(amount_of_used_disk_space + requested.disk_space),
            ))
            .execute(conn)?;

        hosts
            .filter(uuid.eq(&selected_host.uuid))
            .select(HostEntry::as_select())
            .first::<HostEntry>(conn)
    });

    match result {
        Ok(host) => Ok(host),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Releases resources, which were allocated on a host by `allocate_host_resources`.
///
/// The used values of the host are decreased by the given values, but never below 0.
///
/// # Arguments
/// * `host_uuid` - Unique identifier of the host
/// * `released` - Resources to release on the host
///
/// # Returns
/// * Ok(()) if the resources were successfully released
/// * DbError::NotFound if the host doesn't exist
/// * DbError::InternalError if there was an error executing the query
pub fn release_host_resources(
    host_uuid: &Uuid,
    released: &HostResources,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::hosts::dsl::*;
    use diesel::dsl::sql;
    use diesel::sql_types::BigInt;

    // MAX(x, 0) keeps the values valid for the CHECK-constraints of the table
    match diesel::update(hosts.filter(uuid.eq(host_uuid.to_string())))
        .set((
            used_number_of_cores.eq(sql::<BigInt>("MAX(used_number_of_cores - ")
                .bind::<BigInt, _>(released.number_of_cores)
                .sql(", 0)")),
            amount_of_used_memory.eq(sql::<BigInt>("MAX(amount_of_used_memory - ")
                .bind::<BigInt, _>(released.memory_size)
                .sql(", 0)")),
            amount_of_used_disk_space.eq(sql::<BigInt>("MAX(amount_of_used_disk_space - ")
                .bind::<BigInt, _>(released.disk_space)
                .sql(", 0)")),
        ))
        .execute(&mut *conn)
    {
        Ok(0) => Err(enums::DbError::NotFound),
        Ok(_) => Ok(()),
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
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
                assert_eq!(retrieved_host.number_of_cores, host.number_of_cores);
                assert_eq!(retrieved_host.memory_size, host.memory_size);
                assert_eq!(retrieved_host.disk_space, host.disk_space);
                assert_eq!(retrieved_host.used_number_of_cores, 0);
                assert_eq!(retrieved_host.amount_of_used_memory, 0);
                assert_eq!(retrieved_host.amount_of_used_disk_space, 0);
            }
            Err(_) => {
                assert_eq!(true, false);
            }
        };

        hard_delete_host(&uuid1);
    }

    #[test]
    #[serial]
    fn test_add_new_host_and_update_resources() {
        let _ = init_host_table();
        let uuid1 = Uuid::new_v4();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        hard_delete_host(&uuid1);

        let resources = HostResources {
            number_of_cores: 8,
            memory_size: 16384,
            disk_space: 512,
        };
        add_new_host(
            &uuid1,
            "Alice",
            "http://127.0.0.1:11420",
            &resources,
            &context,
        )
        .unwrap();

        let Ok(retrieved_host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(retrieved_host.number_of_cores, 8);
        assert_eq!(retrieved_host.memory_size, 16384);
        assert_eq!(retrieved_host.disk_space, 512);
        assert_eq!(retrieved_host.used_number_of_cores, 0);
        assert_eq!(retrieved_host.amount_of_used_memory, 0);
        assert_eq!(retrieved_host.amount_of_used_disk_space, 0);

        let new_resources = HostResources {
            number_of_cores: 32,
            memory_size: 65536,
            disk_space: 2048,
        };
        assert!(update_host_resources(&uuid1, &new_resources, &context).is_ok());

        let Ok(retrieved_host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(retrieved_host.number_of_cores, 32);
        assert_eq!(retrieved_host.memory_size, 65536);
        assert_eq!(retrieved_host.disk_space, 2048);

        // negative values are rejected by the database
        let invalid_resources = HostResources {
            number_of_cores: -1,
            memory_size: 0,
            disk_space: 0,
        };
        assert!(update_host_resources(&uuid1, &invalid_resources, &context).is_err());

        hard_delete_host(&uuid1);
        assert!(update_host_resources(&uuid1, &new_resources, &context).is_err());
    }

    #[test]
    #[serial]
    fn test_allocate_and_release_host_resources() {
        let _ = init_host_table();
        let uuid1 = Uuid::new_v4();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        hard_delete_host(&uuid1);

        // the values are that large, that no other host of the test-database can be selected
        let resources = HostResources {
            number_of_cores: 1_000_000,
            memory_size: 1_000_000_000,
            disk_space: 1_000_000_000,
        };
        add_new_host(
            &uuid1,
            "Alice",
            "http://127.0.0.1:11420",
            &resources,
            &context,
        )
        .unwrap();

        let requested = HostResources {
            number_of_cores: 600_000,
            memory_size: 600_000_000,
            disk_space: 600_000_000,
        };

        // first allocation fits
        let Ok(host) = allocate_host_resources(&requested, &context) else {
            panic!("no host selected");
        };
        assert_eq!(host.uuid, uuid1.to_string());
        assert_eq!(host.used_number_of_cores, 600_000);
        assert_eq!(host.amount_of_used_memory, 600_000_000);
        assert_eq!(host.amount_of_used_disk_space, 600_000_000);

        // second allocation doesn't fit anymore and changes nothing
        assert!(matches!(
            allocate_host_resources(&requested, &context),
            Err(enums::DbError::NotFound)
        ));

        // a single exhausted resource is enough to exclude the host
        let only_disk = HostResources {
            number_of_cores: 1,
            memory_size: 1,
            disk_space: 400_000_001,
        };
        assert!(allocate_host_resources(&only_disk, &context).is_err());

        // the remaining resources fit exactly
        let remaining = HostResources {
            number_of_cores: 400_000,
            memory_size: 400_000_000,
            disk_space: 400_000_000,
        };
        let Ok(host) = allocate_host_resources(&remaining, &context) else {
            panic!("no host selected");
        };
        assert_eq!(host.used_number_of_cores, 1_000_000);
        assert_eq!(host.amount_of_used_memory, 1_000_000_000);
        assert_eq!(host.amount_of_used_disk_space, 1_000_000_000);

        // release the first allocation, so it fits again
        assert!(release_host_resources(&uuid1, &requested).is_ok());
        let Ok(host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(host.used_number_of_cores, 400_000);
        assert_eq!(host.amount_of_used_memory, 400_000_000);
        assert_eq!(host.amount_of_used_disk_space, 400_000_000);
        assert!(allocate_host_resources(&requested, &context).is_ok());

        // releasing more than allocated stops at 0
        assert!(release_host_resources(&uuid1, &resources).is_ok());
        assert!(release_host_resources(&uuid1, &resources).is_ok());
        let Ok(host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(host.used_number_of_cores, 0);
        assert_eq!(host.amount_of_used_memory, 0);
        assert_eq!(host.amount_of_used_disk_space, 0);

        // deleted hosts are not selected
        let _ = delete_host_admin(&uuid1, &context);
        assert!(allocate_host_resources(&requested, &context).is_err());

        hard_delete_host(&uuid1);
    }

    #[test]
    #[serial]
    fn test_allocate_host_resources_parallel() {
        let _ = init_host_table();
        let uuid1 = Uuid::new_v4();

        let context = UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-project".to_string(),
            is_admin: false.to_string(),
            is_project_admin: false.to_string(),
        };

        hard_delete_host(&uuid1);

        // space for exactly 10 allocations of the requested size
        let resources = HostResources {
            number_of_cores: 2_000_000,
            memory_size: 2_000_000_000,
            disk_space: 2_000_000_000,
        };
        add_new_host(
            &uuid1,
            "Alice",
            "http://127.0.0.1:11420",
            &resources,
            &context,
        )
        .unwrap();

        let requested = HostResources {
            number_of_cores: 200_000,
            memory_size: 200_000_000,
            disk_space: 200_000_000,
        };

        // 32 parallel requests, where only 10 are allowed to succeed
        let handles: Vec<_> = (0..32)
            .map(|_| {
                let requested = requested.clone();
                let context = context.clone();
                std::thread::spawn(move || allocate_host_resources(&requested, &context).is_ok())
            })
            .collect();
        let successful = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|ok| *ok)
            .count();
        assert_eq!(successful, 10);

        let Ok(host) = get_host(&uuid1, &context) else {
            panic!("host not found");
        };
        assert_eq!(host.used_number_of_cores, 2_000_000);
        assert_eq!(host.amount_of_used_memory, 2_000_000_000);
        assert_eq!(host.amount_of_used_disk_space, 2_000_000_000);

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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
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
            number_of_cores: 16,
            used_number_of_cores: 0,
            memory_size: 32768,
            amount_of_used_memory: 0,
            disk_space: 1024,
            amount_of_used_disk_space: 0,
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
