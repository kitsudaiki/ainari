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
use diesel::result::DatabaseErrorKind;
use std::error::Error;
use std::net::Ipv4Addr;
use uuid::Uuid;

use crate::database::{assignable_ip_range, db_handle};

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
    );
    DROP INDEX IF EXISTS floating_ips_floating_ip_addr;
    CREATE UNIQUE INDEX IF NOT EXISTS floating_ips_active_floating_ip_addr
        ON floating_ips (floating_ip_addr) WHERE status = 'ACTIVE';",
    )?;

    Ok(())
}

/// Errors, which can occur while reserving a floating IP-address.
#[derive(Debug, PartialEq)]
pub enum FloatingIpReserveError {
    /// The CIDR is invalid or too small to contain at least one assignable IP-address
    InvalidCidr,
    /// The requested floating IP-address is not within the assignable range of the CIDR
    NotInRange,
    /// The requested floating IP-address is already used by another entry
    AlreadyUsed,
    /// All floating IP-addresses of the CIDR are already used
    NoFreeAddress,
    /// Any other database-error
    InternalError,
}

impl From<enums::DbError> for FloatingIpReserveError {
    fn from(_: enums::DbError) -> Self {
        FloatingIpReserveError::InternalError
    }
}

/// Reserves a floating IP-address by adding a new floating-ip entry to the database.
///
/// Floating IP-addresses are unique across all ACTIVE entries of the whole table and not only
/// within a network. Deleted entries don't block their floating IP-address, so it can be reused.
/// The reservation is done by inserting the new entry, so the floating IP-address can not
/// be taken by another request. The status is set to "ACTIVE" and timestamps are set to the
/// current time.
///
/// If a floating IP-address is requested, it is only reserved, if it is within the range of the
/// CIDR and not already used by an ACTIVE entry. Otherwise the new floating IP-address is the highest
/// floating IP-address of all ACTIVE entries within the range of the CIDR increased by one. If another request was faster, the
/// floating IP-address is increased again until the reservation was successful.
///
/// # Arguments
/// * `network_uuid` - The UUID of the network the floating IP-address belongs to
/// * `internal_ip_addr` - The internal IP-address, which is reachable over the floating IP-address
/// * `floating_ip_addr` - Optional requested floating IP-address. If None, a free one is selected.
/// * `floating_cidr` - CIDR like `203.0.113.0/24`, which defines the range of the floating IP-addresses.
///   The network-address, the first address (gateway) and the broadcast-address are not assigned.
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the UUID of the new entry and the reserved floating IP-address,
/// or a FloatingIpReserveError if the reservation failed
pub fn add_new_floating_ip(
    network_uuid: &Uuid,
    internal_ip_addr: &Ipv4Addr,
    floating_ip_addr: Option<&Ipv4Addr>,
    floating_cidr: &str,
    context: &UserContext,
) -> Result<(Uuid, Ipv4Addr), FloatingIpReserveError> {
    let (first_floating_ip, last_floating_ip) = match assignable_ip_range(floating_cidr) {
        Some(range) => range,
        None => {
            log::error!("Invalid or too small CIDR '{floating_cidr}' for floating IP-addresses");
            return Err(FloatingIpReserveError::InvalidCidr);
        }
    };

    // reserve exactly the requested floating IP-address
    if let Some(requested_ip) = floating_ip_addr {
        let requested = u32::from(*requested_ip);
        if !(first_floating_ip..=last_floating_ip).contains(&requested) {
            return Err(FloatingIpReserveError::NotInRange);
        }

        return reserve_floating_ip_from(
            requested,
            last_floating_ip,
            false,
            network_uuid,
            internal_ip_addr,
            context,
        );
    }

    let first_candidate = match get_highest_floating_ip(first_floating_ip, last_floating_ip)? {
        Some(highest) => u32::from(highest) + 1,
        None => first_floating_ip,
    };

    reserve_floating_ip_from(
        first_candidate,
        last_floating_ip,
        true,
        network_uuid,
        internal_ip_addr,
        context,
    )
}

/// Tries to reserve floating IP-addresses, beginning with the given one, until the reservation
/// was successful.
///
/// If the insert fails, because the floating IP-address was already reserved by another request
/// in the meantime, the floating IP-address is increased and the reservation is tried again,
/// if `increase_on_conflict` is set. Otherwise the reservation fails with `AlreadyUsed`.
///
/// # Arguments
/// * `first_candidate` - Numeric value of the first floating IP-address to try
/// * `last_floating_ip` - Numeric value of the last floating IP-address, which can be assigned
/// * `increase_on_conflict` - true to try the next floating IP-address, if the current one is already used
/// * `network_uuid` - The UUID of the network the floating IP-address belongs to
/// * `internal_ip_addr` - The internal IP-address, which is reachable over the floating IP-address
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the UUID of the new entry and the reserved floating IP-address,
/// or a FloatingIpReserveError if the reservation failed
fn reserve_floating_ip_from(
    first_candidate: u32,
    last_floating_ip: u32,
    increase_on_conflict: bool,
    network_uuid: &Uuid,
    internal_ip_addr: &Ipv4Addr,
    context: &UserContext,
) -> Result<(Uuid, Ipv4Addr), FloatingIpReserveError> {
    let mut candidate = first_candidate;
    loop {
        if candidate > last_floating_ip {
            log::error!("No free floating IP-address left");
            return Err(FloatingIpReserveError::NoFreeAddress);
        }

        let floating_ip_uuid = Uuid::new_v4();
        let floating_ip_addr = Ipv4Addr::from(candidate);
        let floating_ip = FloatingIpEntry {
            uuid: floating_ip_uuid,
            network_uuid: *network_uuid,
            internal_ip_addr: *internal_ip_addr,
            floating_ip_addr,
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

        match add_floating_ip(floating_ip) {
            Ok(_) => return Ok((floating_ip_uuid, floating_ip_addr)),
            Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info)) => {
                // another request was faster and reserved this floating IP-address, so try the next one.
                // Otherwise the UUID was in conflict, so only a new UUID is generated in the next try.
                if info.message().contains("floating_ips.floating_ip_addr") {
                    if !increase_on_conflict {
                        return Err(FloatingIpReserveError::AlreadyUsed);
                    }
                    candidate += 1;
                }
            }
            Err(e) => {
                log::error!("Database-error: {e:?}");
                return Err(FloatingIpReserveError::InternalError);
            }
        }
    }
}

/// Gets the highest floating IP-address within a range from the database.
///
/// The whole table is taken into account, because floating IP-addresses are unique across all
/// ACTIVE entries of the whole table. Only ACTIVE entries are taken into account, because deleted
/// entries don't block their floating IP-address. The IP-addresses are compared numerically, because the
/// alphabetical order of the strings is not the numeric order (`10.0.0.10` < `10.0.0.9`).
///
/// # Arguments
/// * `first_floating_ip` - Numeric value of the first IP-address of the range
/// * `last_floating_ip` - Numeric value of the last IP-address of the range
///
/// # Returns
/// A Result containing the highest floating IP-address within the range, or None if there is none yet
fn get_highest_floating_ip(
    first_floating_ip: u32,
    last_floating_ip: u32,
) -> Result<Option<Ipv4Addr>, enums::DbError> {
    let floating_ip_strs = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::floating_ips::dsl::*;

        floating_ips
            .filter(status.eq("ACTIVE"))
            .select(floating_ip_addr)
            .load::<String>(&mut *conn)
            .map_err(|e| {
                log::error!("Database-error: {e:?}");
                enums::DbError::InternalError
            })?
    };

    let mut highest: Option<Ipv4Addr> = None;
    for ip_str in floating_ip_strs {
        let ip = ip_str.parse::<Ipv4Addr>().map_err(|_| {
            log::error!("Invalid floating IP-address '{ip_str}' in the database");
            enums::DbError::InternalError
        })?;
        if (first_floating_ip..=last_floating_ip).contains(&u32::from(ip)) {
            highest = highest.max(Some(ip));
        }
    }

    Ok(highest)
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

    use std::sync::LazyLock;
    use std::sync::atomic::{AtomicU32, Ordering};

    const INTERNAL_IP: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);

    /// CIDR for the reservation-tests, which is not used by the entries of `new_entry`.
    const TEST_CIDR: &str = "203.0.113.0/24";
    const FIRST_TEST_IP: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 2);
    const LAST_TEST_IP: Ipv4Addr = Ipv4Addr::new(203, 0, 113, 254);

    /// Random start within `198.18.0.0/15` for the floating IP-addresses of the test-entries,
    /// so leftovers of an aborted test-run in the database don't collide with a new run.
    static FLOATING_IP_BASE: LazyLock<u32> = LazyLock::new(|| {
        u32::from(Ipv4Addr::new(198, 18, 0, 0)) + u32::from(rand::random::<u16>())
    });
    static FLOATING_IP_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Returns a new floating IP-address for a test-entry, because floating IP-addresses
    /// have to be unique within the whole table.
    fn next_floating_ip() -> Ipv4Addr {
        Ipv4Addr::from(*FLOATING_IP_BASE + FLOATING_IP_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    fn hard_delete_floating_ip(floating_ip_uuid: &Uuid) {
        use self::floating_ips::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(floating_ips.filter(uuid.eq(floating_ip_uuid.to_string())))
            .execute(&mut *conn);
    }

    fn hard_delete_floating_ip_addr(address: &Ipv4Addr) {
        use self::floating_ips::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(floating_ips.filter(floating_ip_addr.eq(address.to_string())))
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
            floating_ip_addr: next_floating_ip(),
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
    fn test_add_new_floating_ip() {
        let _ = init_floating_ip_table();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let (uuid1, floating_ip1) =
            add_new_floating_ip(&network_uuid1, &INTERNAL_IP, None, TEST_CIDR, &context)
                .unwrap_or_else(|_| panic!("reservation failed"));
        assert!(
            (u32::from(FIRST_TEST_IP)..=u32::from(LAST_TEST_IP)).contains(&u32::from(floating_ip1))
        );

        let retrieved = expect_entry(get_floating_ip(&uuid1, &context));
        assert_eq!(retrieved.uuid, uuid1);
        assert_eq!(retrieved.network_uuid, network_uuid1);
        assert_eq!(retrieved.internal_ip_addr, INTERNAL_IP);
        assert_eq!(retrieved.floating_ip_addr, floating_ip1);
        assert_eq!(retrieved.owner_id, "test-user");
        assert_eq!(retrieved.status, "ACTIVE");

        // floating IP-addresses are unique within the whole table, so another network gets the next one
        let (uuid2, floating_ip2) =
            add_new_floating_ip(&network_uuid2, &INTERNAL_IP, None, TEST_CIDR, &context)
                .unwrap_or_else(|_| panic!("reservation failed"));
        assert_ne!(uuid2, uuid1);
        assert_eq!(u32::from(floating_ip2), u32::from(floating_ip1) + 1);

        // deleted entries don't block their floating IP-address, so it is reused
        assert!(delete_floating_ip(&uuid2, &context).is_ok());
        let (uuid3, floating_ip3) =
            add_new_floating_ip(&network_uuid2, &INTERNAL_IP, None, TEST_CIDR, &context)
                .unwrap_or_else(|_| panic!("reservation failed"));
        assert_ne!(uuid3, uuid2);
        assert_eq!(floating_ip3, floating_ip2);

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&uuid3);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_floating_ip_addr() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        let entry1 = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let mut entry2 = new_entry(
            &uuid2,
            &network_uuid2,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry2.floating_ip_addr = entry1.floating_ip_addr;

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);

        add_floating_ip(entry1).unwrap();
        // the floating IP-address must be unique, even across networks
        assert!(add_floating_ip(entry2).is_err());

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
    }

    #[test]
    #[serial]
    fn test_duplicate_floating_ip_addr_of_deleted_entries() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let uuid4 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry1 = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        let floating_ip = entry1.floating_ip_addr;
        let mut entry2 = new_entry(
            &uuid2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        entry2.floating_ip_addr = floating_ip;
        let mut entry3 = new_entry(
            &uuid3,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry3.floating_ip_addr = floating_ip;
        let mut entry4 = new_entry(
            &uuid4,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry4.floating_ip_addr = floating_ip;

        for entry_uuid in [&uuid1, &uuid2, &uuid3, &uuid4] {
            hard_delete_floating_ip(entry_uuid);
        }

        // deleted entries can share a floating IP-address with each other and with one ACTIVE entry
        add_floating_ip(entry1).unwrap();
        add_floating_ip(entry2).unwrap();
        add_floating_ip(entry3).unwrap();
        // but not with a second ACTIVE entry
        assert!(add_floating_ip(entry4).is_err());

        // after deleting the ACTIVE entry, the floating IP-address can be used again
        assert!(delete_floating_ip(&uuid3, &context).is_ok());
        let mut entry4 = new_entry(
            &uuid4,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry4.floating_ip_addr = floating_ip;
        add_floating_ip(entry4).unwrap();

        for entry_uuid in [&uuid1, &uuid2, &uuid3, &uuid4] {
            hard_delete_floating_ip(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_get_highest_floating_ip() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let uuid4 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        // an own range, so no other entries of the table are within it
        let first = u32::from(Ipv4Addr::new(192, 0, 2, 2));
        let last = u32::from(Ipv4Addr::new(192, 0, 2, 254));

        // alphabetically "192.0.2.9" would be higher than "192.0.2.10"
        let mut entry1 = new_entry(
            &uuid1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry1.floating_ip_addr = Ipv4Addr::new(192, 0, 2, 9);
        let mut entry2 = new_entry(
            &uuid2,
            &network_uuid2,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry2.floating_ip_addr = Ipv4Addr::new(192, 0, 2, 10);
        // deleted, so it is ignored
        let mut entry4 = new_entry(
            &uuid4,
            &network_uuid2,
            "test-user",
            "test-project",
            "DELETED",
        );
        entry4.floating_ip_addr = Ipv4Addr::new(192, 0, 2, 20);
        // outside of the range, so it is ignored
        let mut entry3 = new_entry(
            &uuid3,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry3.floating_ip_addr = Ipv4Addr::new(192, 0, 3, 50);

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&uuid3);
        hard_delete_floating_ip(&uuid4);

        assert!(matches!(get_highest_floating_ip(first, last), Ok(None)));

        add_floating_ip(entry1).unwrap();
        add_floating_ip(entry2).unwrap();
        add_floating_ip(entry3).unwrap();
        add_floating_ip(entry4).unwrap();

        assert!(matches!(
            get_highest_floating_ip(first, last),
            Ok(Some(ip)) if ip == Ipv4Addr::new(192, 0, 2, 10)
        ));

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&uuid3);
        hard_delete_floating_ip(&uuid4);
    }

    #[test]
    #[serial]
    fn test_reserve_floating_ip_retry_on_conflict() {
        let _ = init_floating_ip_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // simulate other requests in another network, which already reserved the first two addresses
        let base = u32::from(Ipv4Addr::new(192, 0, 2, 100));
        let mut taken1 = new_entry(
            &uuid1,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        );
        taken1.floating_ip_addr = Ipv4Addr::from(base);
        let mut taken2 = new_entry(
            &uuid2,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        );
        taken2.floating_ip_addr = Ipv4Addr::from(base + 1);

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        add_floating_ip(taken1).unwrap();
        add_floating_ip(taken2).unwrap();

        let (reserved_uuid, reserved_ip) = reserve_floating_ip_from(
            base,
            base + 10,
            true,
            &network_uuid1,
            &INTERNAL_IP,
            &context,
        )
        .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(reserved_ip, Ipv4Addr::from(base + 2));
        assert_eq!(
            expect_entry(get_floating_ip(&reserved_uuid, &context)).network_uuid,
            network_uuid1
        );

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
        hard_delete_floating_ip(&reserved_uuid);
    }

    #[test]
    #[serial]
    fn test_reserve_floating_ip_range_exhausted() {
        let _ = init_floating_ip_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let last = u32::from(LAST_TEST_IP);
        let result =
            reserve_floating_ip_from(last + 1, last, true, &network_uuid1, &INTERNAL_IP, &context);
        assert_eq!(result, Err(FloatingIpReserveError::NoFreeAddress));
    }

    #[test]
    #[serial]
    fn test_add_new_floating_ip_requested() {
        let _ = init_floating_ip_table();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // an own range, so no other entries of the table are within it
        let cidr = "192.0.2.0/24";
        let requested = Ipv4Addr::new(192, 0, 2, 200);
        hard_delete_floating_ip_addr(&requested);

        let (uuid1, floating_ip1) = add_new_floating_ip(
            &network_uuid1,
            &INTERNAL_IP,
            Some(&requested),
            cidr,
            &context,
        )
        .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(floating_ip1, requested);
        assert_eq!(
            expect_entry(get_floating_ip(&uuid1, &context)).floating_ip_addr,
            requested
        );

        // already used by an ACTIVE entry, also in another network
        let result = add_new_floating_ip(
            &network_uuid2,
            &INTERNAL_IP,
            Some(&requested),
            cidr,
            &context,
        );
        assert_eq!(result, Err(FloatingIpReserveError::AlreadyUsed));

        // after deleting the entry, the floating IP-address can be requested again
        assert!(delete_floating_ip(&uuid1, &context).is_ok());
        let (uuid2, floating_ip2) = add_new_floating_ip(
            &network_uuid2,
            &INTERNAL_IP,
            Some(&requested),
            cidr,
            &context,
        )
        .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(floating_ip2, requested);

        // not within the assignable range of the CIDR
        for not_in_range in [
            Ipv4Addr::new(192, 0, 2, 0),
            Ipv4Addr::new(192, 0, 2, 1),
            Ipv4Addr::new(192, 0, 2, 255),
            Ipv4Addr::new(192, 0, 3, 5),
        ] {
            let result = add_new_floating_ip(
                &network_uuid1,
                &INTERNAL_IP,
                Some(&not_in_range),
                cidr,
                &context,
            );
            assert_eq!(result, Err(FloatingIpReserveError::NotInRange));
        }

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);
    }

    #[test]
    #[serial]
    fn test_add_new_floating_ip_invalid_cidr() {
        let _ = init_floating_ip_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        for cidr in ["203.0.113.0/31", "203.0.113.0", "no-cidr/24"] {
            let result = add_new_floating_ip(&network_uuid1, &INTERNAL_IP, None, cidr, &context);
            assert_eq!(result, Err(FloatingIpReserveError::InvalidCidr));
        }
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
        let floating_ip1 = entry1.floating_ip_addr;

        hard_delete_floating_ip(&uuid1);
        hard_delete_floating_ip(&uuid2);

        add_floating_ip(entry1).unwrap();
        add_floating_ip(entry2).unwrap();

        // only the ACTIVE entry is listed
        let entries = list_floating_ips(&context).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, uuid1);
        assert_eq!(entries[0].internal_ip_addr, INTERNAL_IP);
        assert_eq!(entries[0].floating_ip_addr, floating_ip1);

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
