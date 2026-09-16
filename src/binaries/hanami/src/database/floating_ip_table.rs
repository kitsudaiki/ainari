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
use std::net::Ipv4Addr;
use uuid::Uuid;

use crate::database::db_handle;

use ainari_api_structs::user_context::UserContext;
use ainari_common::enums;
use ainari_common::objects::*;

// Define the schema for floating_ips table
table! {
    floating_ips (uuid) {
        uuid -> Varchar,
        network_uuid -> Varchar,
        internal_ip_addr -> Varchar,
        floating_ip_addr -> Varchar,
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

/// Represents an entry in the floating_ips table.
/// This struct contains all the fields required to create, query, and update meta floating_ip_addr records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = floating_ips)]
pub struct FloatingIpEntry {
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub network_uuid: Uuid,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub internal_ip_addr: Ipv4Addr,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub floating_ip_addr: Ipv4Addr,
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

/// Initializes the floating_ips table in the database if it doesn't exist.
///
/// This function creates the table with the appropriate schema and constraints.
/// It's typically called during application startup to ensure the required tables exist.
pub fn init_floating_ip_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS floating_ips (
        uuid VARCHAR(40) PRIMARY KEY,
        network_uuid VARCHAR(40),
        internal_ip_addr VARCHAR(40),
        floating_ip_addr VARCHAR(40),
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

/// Adds a new meta floating_ip_addr to the database.
///
/// This function creates a new FloatingIpEntry with the provided parameters and inserts it into the database.
/// The status is set to "ACTIVE" and timestamps are set to the current time.
///
/// # Arguments
/// * `floating_ip_uuid` - The unique identifier for the meta floating_ip_addr
/// * `floating_ip_name` - The name of the meta floating_ip_addr
/// * `sakura_host_uuid` - The UUID of the Sakura host associated with this floating_ip_addr
/// * `proxy_uuid` - The UUID of the proxy associated with this floating_ip_addr
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_floating_ip(
    floating_ip_uuid: &Uuid,
    network_uuid: &Uuid,
    internal_ip_addr: &Ipv4Addr,
    floating_ip_addr: &Ipv4Addr,
    context: &UserContext,
) -> QueryResult<usize> {
    let floating_ip_addr = FloatingIpEntry {
        uuid: *network_uuid,
        network_uuid: *floating_ip_uuid,
        internal_ip_addr: *internal_ip_addr,
        floating_ip_addr: *floating_ip_addr,
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

    add_floating_ip(floating_ip_addr)
}

/// Adds a meta floating_ip_addr to the database.
///
/// This is a helper function that performs the actual insertion of a FloatingIpEntry into the database.
///
/// # Arguments
/// * `floating_ip_addr` - The FloatingIpEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_floating_ip(floating_ip: FloatingIpEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;
    diesel::insert_into(floating_ips)
        .values(floating_ip)
        .execute(&mut *conn)
}

/// Retrieves a meta floating_ip_addr from the database.
///
/// This function queries the database for a meta floating_ip_addr with the specified UUID and checks the user's permissions.
/// Only active floating_ips are returned, and the query is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `floating_ip_uuid` - The UUID of the meta floating_ip_addr to retrieve
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result containing the FloatingIpEntry if found, or a DbError if not found or an error occurs
pub fn get_floating_ip(
    floating_ip_uuid: &Uuid,
    context: &UserContext,
) -> Result<FloatingIpEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;

    let mut query = floating_ips
        .filter(
            uuid.eq(floating_ip_uuid.to_string())
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
        .select(FloatingIpEntry::as_select())
        .first::<FloatingIpEntry>(&mut *conn)
    {
        Ok(floating_ip) => Ok(floating_ip),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all meta floating_ips that the user has access to.
///
/// This function retrieves all active meta floating_ips and applies permission-based filtering.
/// The results are filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing a vector of FloatingIpEntry objects
#[allow(dead_code)]
pub fn list_floating_ips(context: &UserContext) -> QueryResult<Vec<FloatingIpEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;

    let mut query = floating_ips.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    if context.is_admin != true.to_string() {
        query = query.filter(project_id.eq(context.project_id.clone()));
        if context.is_project_admin != true.to_string() {
            query = query.filter(owner_id.eq(context.user_id.clone()));
        }
    }

    query.select(FloatingIpEntry::as_select()).load(&mut *conn)
}

/// Counts the number of meta floating_ips that the user has access to.
///
/// This function counts all active meta floating_ips and applies permission-based filtering.
/// The count is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing the count of meta floating_ips as an i64
pub fn count_floating_ips(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;

    let mut query = floating_ips.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Force deletes a meta floating_ip_addr from the database.
///
/// This function marks a meta floating_ip_addr as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Arguments
/// * `floating_ip_uuid` - The UUID of the meta floating_ip_addr to delete
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn force_delete_floating_ip(floating_ip_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;
    match diesel::update(floating_ips.filter(uuid.eq(floating_ip_uuid.to_string())))
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

/// Deletes a meta floating_ip_addr from the database.
///
/// This function marks a meta floating_ip_addr as deleted after verifying that the user has permission to delete it.
/// It first checks if the floating_ip_addr exists and if the user has the necessary permissions.
///
/// # Arguments
/// * `floating_ip_uuid` - The UUID of the meta floating_ip_addr to delete
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_floating_ip(
    floating_ip_uuid: &Uuid,
    context: &UserContext,
) -> Result<(), enums::DbError> {
    // Verify the meta floating_ip_addr exists and the user has permission to delete it
    get_floating_ip(floating_ip_uuid, context)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;
    match diesel::update(floating_ips.filter(uuid.eq(floating_ip_uuid.to_string())))
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

/// Deletes all meta floating_ips from the database.
///
/// This function marks all active meta floating_ips as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_all_floating_ip() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::floating_ips::dsl::*;
    match diesel::update(floating_ips.filter(status.eq("ACTIVE")))
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

    const INTERNAL_IP: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
    const FLOATING_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 1);

    fn hard_delete_floating_ip(floating_ip_uuid: &Uuid) {
        use self::floating_ips::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(floating_ips.filter(uuid.eq(floating_ip_uuid.to_string())))
            .execute(&mut *conn);
    }

    /// Builds a FloatingIpEntry for the tests with the given identity and status.
    fn new_entry(
        entry_uuid: &Uuid,
        entry_network_uuid: &Uuid,
        entry_owner_id: &str,
        entry_project_id: &str,
        entry_status: &str,
    ) -> FloatingIpEntry {
        FloatingIpEntry {
            uuid: *entry_uuid,
            network_uuid: *entry_network_uuid,
            internal_ip_addr: INTERNAL_IP,
            floating_ip_addr: FLOATING_IP,
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
    fn expect_entry(result: Result<FloatingIpEntry, enums::DbError>) -> FloatingIpEntry {
        match result {
            Ok(entry) => entry,
            Err(_) => panic!("floating-ip was not found"),
        }
    }

    /// Asserts that a get-result reports a missing entry.
    fn assert_not_found(result: Result<FloatingIpEntry, enums::DbError>) {
        assert!(matches!(result, Err(enums::DbError::NotFound)));
    }

    #[test]
    #[serial]
    fn test_add_get_floating_ip() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_floating_ip(&uuid1);

        add_floating_ip(entry.clone()).unwrap();
        let retrieved = expect_entry(get_floating_ip(&uuid1, &context));

        assert_eq!(retrieved.uuid, entry.uuid);
        assert_eq!(retrieved.network_uuid, entry.network_uuid);
        assert_eq!(retrieved.internal_ip_addr, entry.internal_ip_addr);
        assert_eq!(retrieved.floating_ip_addr, entry.floating_ip_addr);
        assert_eq!(retrieved.owner_id, entry.owner_id);
        assert_eq!(retrieved.project_id, entry.project_id);
        assert_eq!(retrieved.status, entry.status);
        assert_eq!(retrieved.created_at, entry.created_at);
        assert_eq!(retrieved.created_by, entry.created_by);
        assert_eq!(retrieved.updated_at, entry.updated_at);
        assert_eq!(retrieved.updated_by, entry.updated_by);
        assert_eq!(retrieved.deleted_at, entry.deleted_at);
        assert_eq!(retrieved.deleted_by, entry.deleted_by);

        hard_delete_floating_ip(&uuid1);
    }

    #[test]
    #[serial]
    fn test_get_floating_ip_not_found() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        hard_delete_floating_ip(&uuid1);

        assert_not_found(get_floating_ip(&uuid1, &context));
    }

    #[test]
    #[serial]
    fn test_list_floating_ips() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry1 = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);

        add_floating_ip(entry1).unwrap();
        add_floating_ip(entry2).unwrap();

        // only the ACTIVE entry is listed
        let entries = list_floating_ips(&context).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, uuid1);
        assert_eq!(entries[0].internal_ip_addr, INTERNAL_IP);
        assert_eq!(entries[0].floating_ip_addr, FLOATING_IP);

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
    }

    #[test]
    #[serial]
    fn test_delete_floating_ip() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_floating_ip(&uuid1);

        add_floating_ip(entry).unwrap();
        assert!(delete_floating_ip(&uuid1, &context).is_ok());

        assert_not_found(get_floating_ip(&uuid1, &context));

        hard_delete_floating_ip(&uuid1);
    }

    #[test]
    #[serial]
    fn test_force_delete_floating_ip() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // the entry belongs to another project, so only the force-delete can remove it
        let entry = new_entry(
            &uuid1,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );

        hard_delete_floating_ip(&uuid1);

        add_floating_ip(entry).unwrap();
        assert!(force_delete_floating_ip(&uuid1).is_ok());

        assert_not_found(get_floating_ip(&uuid1, &context));

        hard_delete_floating_ip(&uuid1);
    }

    #[test]
    #[serial]
    fn test_delete_all_floating_ip() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", true, false);

        let entry1 = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);

        add_floating_ip(entry1).unwrap();
        add_floating_ip(entry2).unwrap();

        assert!(delete_all_floating_ip().is_ok());

        assert_eq!(list_floating_ips(&context).unwrap().len(), 0);

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
    }

    #[test]
    #[serial]
    fn test_count_floating_ips() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry1 = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        // neither DELETED entries nor entries of other owners are counted
        let entry3 = new_entry(
            &uuid3,
            &network_uuid1,
            "other-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&uuid3);

        add_floating_ip(entry1).unwrap();
        add_floating_ip(entry2).unwrap();
        add_floating_ip(entry3).unwrap();

        assert_eq!(count_floating_ips(&context).unwrap(), 2);

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&uuid3);
    }

    #[test]
    #[serial]
    fn test_floating_ips_permissions() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user-42",
            "test_permissions_1",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            &network_uuid1,
            "test-user-43",
            "test_permissions_1",
            "ACTIVE",
        );
        let entry3 = new_entry(
            &uuid3,
            &network_uuid1,
            "test-user-44",
            "test_permissions_2",
            "ACTIVE",
        );

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&uuid3);

        add_floating_ip(entry1).unwrap();
        add_floating_ip(entry2).unwrap();
        add_floating_ip(entry3).unwrap();

        // list-test normal user
        let context = new_context("test-user-42", "test_permissions_1", false, false);
        assert_eq!(list_floating_ips(&context).unwrap().len(), 1);

        // list-test project-admin
        let context = new_context("test-user-42", "test_permissions_1", false, true);
        assert_eq!(list_floating_ips(&context).unwrap().len(), 2);

        // list-test admin
        let context = new_context("test-user-42", "test_permissions_1", true, false);
        assert_eq!(list_floating_ips(&context).unwrap().len(), 3);

        // get-test normal user
        let context = new_context("test-user-42", "test_permissions_1", false, false);
        let retrieved = expect_entry(get_floating_ip(&uuid1, &context));
        assert_eq!(retrieved.uuid, uuid1);

        // get-test normal user, entry of another user within the same project
        assert!(get_floating_ip(&uuid2, &context).is_err());

        // get-test normal user, entry of another project
        assert!(get_floating_ip(&uuid3, &context).is_err());

        // get-test project-admin, entry of another user within the same project
        let context = new_context("test-user-42", "test_permissions_1", false, true);
        let retrieved = expect_entry(get_floating_ip(&uuid2, &context));
        assert_eq!(retrieved.uuid, uuid2);

        // get-test admin, entry of another project
        let context = new_context("test-user-42", "test_permissions_1", true, false);
        let retrieved = expect_entry(get_floating_ip(&uuid3, &context));
        assert_eq!(retrieved.uuid, uuid3);

        // delete-test normal user, entry of another project
        let context = new_context("test-user-42", "test_permissions_1", false, false);
        assert!(delete_floating_ip(&uuid3, &context).is_err());

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&uuid3);
    }
}
