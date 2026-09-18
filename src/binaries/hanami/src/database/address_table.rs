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

/// Prefix of all generated MAC-addresses. The `02` marks them as locally administered
/// unicast addresses, so they can not collide with vendor-assigned addresses.
const MAC_ADDRESS_PREFIX: &str = "02:";

/// Numeric value of the first generated MAC-address `02:00:00:00:00:01`.
const FIRST_MAC_ADDRESS: u64 = 0x02_00_00_00_00_01;

/// Numeric value of the last possible MAC-address `02:ff:ff:ff:ff:ff`.
const LAST_MAC_ADDRESS: u64 = 0x02_ff_ff_ff_ff_ff;

// Define the schema for addresses table
table! {
    addresses (mac_address) {
        mac_address -> Varchar,
        internal_ip -> Varchar,
        floating_ip -> Varchar,
        network_uuid -> Varchar,
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

/// Represents an entry in the addresses table.
/// This struct contains all the fields required to create, query, and update address records.
#[derive(Insertable, Queryable, Selectable, Debug, PartialEq, Clone)]
#[diesel(table_name = addresses)]
pub struct AddressEntry {
    pub mac_address: String,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub internal_ip: Ipv4Addr,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub floating_ip: Ipv4Addr,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub network_uuid: Uuid,
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

/// Initializes the addresses table in the database if it doesn't exist.
///
/// This function creates the table with the appropriate schema and constraints.
/// It's typically called during application startup to ensure the required tables exist.
pub fn init_address_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS addresses (
        mac_address VARCHAR(40) PRIMARY KEY,
        internal_ip VARCHAR(40),
        floating_ip VARCHAR(40),
        network_uuid VARCHAR(40),
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
    CREATE UNIQUE INDEX IF NOT EXISTS addresses_network_internal_ip
        ON addresses (network_uuid, internal_ip);",
    )?;

    Ok(())
}

/// Reserves a new MAC-address and a new internal IP-address by adding a new entry.
///
/// The new MAC-address is the highest existing MAC-address increased by one, and the new internal
/// IP-address is the highest existing internal IP-address within the same network increased by one.
/// The reservation is done by inserting the new entry, so neither of them can be taken by another
/// request. If another request was faster, the conflicting value is increased again until the
/// reservation was successful.
///
/// # Arguments
/// * `floating_ip` - The floating IP-address assigned to the MAC-address
/// * `network_uuid` - The UUID of the network the address belongs to
/// * `internal_cidr` - CIDR of the network like `192.168.100.0/24`, which defines the range of the
///   internal IP-addresses. The network-address, the first address (gateway) and the broadcast-address
///   are not assigned.
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the reserved MAC-address and internal IP-address, or a DbError if the reservation failed
#[allow(dead_code)]
pub fn reserve_new_address(
    floating_ip: &Ipv4Addr,
    network_uuid: &Uuid,
    internal_cidr: &str,
    context: &UserContext,
) -> Result<(String, Ipv4Addr), enums::DbError> {
    let (first_internal_ip, last_internal_ip) = match assignable_ip_range(internal_cidr) {
        Some(range) => range,
        None => {
            log::error!("Invalid or too small CIDR '{internal_cidr}' for internal IP-addresses");
            return Err(enums::DbError::InternalError);
        }
    };

    let highest_mac_address = get_highest_mac_address().map_err(|e| {
        log::error!("Database-error: {e:?}");
        enums::DbError::InternalError
    })?;

    let first_mac_candidate = match highest_mac_address {
        Some(highest) => match mac_address_to_number(&highest) {
            Some(number) => number + 1,
            None => {
                log::error!("Invalid MAC-address '{highest}' in the database");
                return Err(enums::DbError::InternalError);
            }
        },
        None => FIRST_MAC_ADDRESS,
    };

    // start behind the highest internal IP-address, but never below the range of the CIDR
    let first_ip_candidate = match get_highest_internal_ip(network_uuid)? {
        Some(highest) => (u32::from(highest) + 1).max(first_internal_ip),
        None => first_internal_ip,
    };

    reserve_address_from(
        first_mac_candidate,
        first_ip_candidate,
        last_internal_ip,
        floating_ip,
        network_uuid,
        context,
    )
}

/// Tries to reserve MAC-addresses and internal IP-addresses, beginning with the given ones,
/// until the reservation was successful.
///
/// If the insert fails, because the MAC-address or the internal IP-address was already reserved
/// by another request in the meantime, the conflicting value is increased and the reservation is
/// tried again.
///
/// # Arguments
/// * `first_mac_candidate` - Numeric value of the first MAC-address to try
/// * `first_ip_candidate` - Numeric value of the first internal IP-address to try
/// * `last_internal_ip` - Numeric value of the last internal IP-address, which can be assigned
/// * `floating_ip` - The floating IP-address assigned to the MAC-address
/// * `network_uuid` - The UUID of the network the address belongs to
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the reserved MAC-address and internal IP-address, or a DbError if the reservation failed
fn reserve_address_from(
    first_mac_candidate: u64,
    first_ip_candidate: u32,
    last_internal_ip: u32,
    floating_ip: &Ipv4Addr,
    network_uuid: &Uuid,
    context: &UserContext,
) -> Result<(String, Ipv4Addr), enums::DbError> {
    let mut mac_candidate = first_mac_candidate;
    let mut ip_candidate = first_ip_candidate;
    loop {
        if mac_candidate > LAST_MAC_ADDRESS {
            log::error!(
                "No free MAC-address left in the range of the prefix '{MAC_ADDRESS_PREFIX}'"
            );
            return Err(enums::DbError::InternalError);
        }
        if ip_candidate > last_internal_ip {
            log::error!("No free internal IP-address left in network '{network_uuid}'");
            return Err(enums::DbError::InternalError);
        }

        let new_mac_address = number_to_mac_address(mac_candidate);
        let new_internal_ip = Ipv4Addr::from(ip_candidate);
        match add_new_address(
            &new_mac_address,
            &new_internal_ip,
            floating_ip,
            network_uuid,
            context,
        ) {
            Ok(_) => return Ok((new_mac_address, new_internal_ip)),
            // another request was faster and reserved one of the values, so try the next one
            Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info)) => {
                let message = info.message();
                let mac_conflict = message.contains("addresses.mac_address");
                let ip_conflict = message.contains("addresses.internal_ip");
                if mac_conflict || !ip_conflict {
                    mac_candidate += 1;
                }
                if ip_conflict || !mac_conflict {
                    ip_candidate += 1;
                }
            }
            Err(e) => {
                log::error!("Database-error: {e:?}");
                return Err(enums::DbError::InternalError);
            }
        }
    }
}

/// Gets the highest internal IP-address within a network from the database.
///
/// Entries of all states are taken into account, because also deleted entries still block
/// their internal IP-address. The IP-addresses are compared numerically, because the
/// alphabetical order of the strings is not the numeric order (`10.0.0.10` < `10.0.0.9`).
///
/// # Arguments
/// * `address_network_uuid` - The UUID of the network
///
/// # Returns
/// A Result containing the highest internal IP-address, or None if the network has no address yet
fn get_highest_internal_ip(
    address_network_uuid: &Uuid,
) -> Result<Option<Ipv4Addr>, enums::DbError> {
    let internal_ips = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::addresses::dsl::*;

        addresses
            .filter(network_uuid.eq(address_network_uuid.to_string()))
            .select(internal_ip)
            .load::<String>(&mut *conn)
            .map_err(|e| {
                log::error!("Database-error: {e:?}");
                enums::DbError::InternalError
            })?
    };

    let mut highest: Option<Ipv4Addr> = None;
    for ip_str in internal_ips {
        let ip = ip_str.parse::<Ipv4Addr>().map_err(|_| {
            log::error!("Invalid internal IP-address '{ip_str}' in the database");
            enums::DbError::InternalError
        })?;
        highest = highest.max(Some(ip));
    }

    Ok(highest)
}

/// Converts a MAC-address-string like `02:00:00:00:00:2a` into its numeric value.
///
/// # Returns
/// The numeric value, or None if the string is not a valid MAC-address
fn mac_address_to_number(address_mac: &str) -> Option<u64> {
    let octets: Vec<&str> = address_mac.split(':').collect();
    if octets.len() != 6 {
        return None;
    }

    let mut number: u64 = 0;
    for octet in octets {
        if octet.len() != 2 {
            return None;
        }
        number = (number << 8) | u64::from(u8::from_str_radix(octet, 16).ok()?);
    }

    Some(number)
}

/// Converts a numeric value into a MAC-address-string like `02:00:00:00:00:2a`.
///
/// The octets are written as lower-case hex-values with a fixed width, so the
/// alphabetical order of the strings is the same as the numeric order.
fn number_to_mac_address(number: u64) -> String {
    let bytes = number.to_be_bytes();
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
    )
}

/// Gets the highest generated MAC-address from the database.
///
/// Entries of all states are taken into account, because also deleted entries still
/// block the MAC-address as primary key of the table.
///
/// # Returns
/// A QueryResult containing the highest MAC-address, or None if the table has no generated MAC-address yet
fn get_highest_mac_address() -> QueryResult<Option<String>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    addresses
        .filter(mac_address.like(format!("{MAC_ADDRESS_PREFIX}%")))
        .select(mac_address)
        .order(mac_address.desc())
        .first::<String>(&mut *conn)
        .optional()
}

/// Adds a new address to the database.
///
/// This function creates a new AddressEntry with the provided parameters and inserts it into the database.
/// The status is set to "ACTIVE" and timestamps are set to the current time.
///
/// # Arguments
/// * `mac_address` - The MAC-address, which identifies the address-entry
/// * `internal_ip` - The internal IP-address assigned to the MAC-address
/// * `floating_ip` - The floating IP-address assigned to the MAC-address
/// * `network_uuid` - The UUID of the network the address belongs to
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult indicating the number of rows affected
pub fn add_new_address(
    mac_address: &str,
    internal_ip: &Ipv4Addr,
    floating_ip: &Ipv4Addr,
    network_uuid: &Uuid,
    context: &UserContext,
) -> QueryResult<usize> {
    let address = AddressEntry {
        mac_address: mac_address.to_string(),
        internal_ip: *internal_ip,
        floating_ip: *floating_ip,
        network_uuid: *network_uuid,
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

    add_address(address)
}

/// Adds an address to the database.
///
/// This is a helper function that performs the actual insertion of an AddressEntry into the database.
///
/// # Arguments
/// * `address` - The AddressEntry to be inserted
///
/// # Returns
/// A QueryResult indicating the number of rows affected
#[allow(dead_code)]
pub fn add_address(address: AddressEntry) -> QueryResult<usize> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    diesel::insert_into(addresses)
        .values(address)
        .execute(&mut *conn)
}

/// Retrieves an address from the database.
///
/// This function queries the database for an address with the specified MAC-address.
/// Only active addresses are returned. There is no permission-based filtering.
///
/// # Arguments
/// * `address_mac` - The MAC-address of the address to retrieve
///
/// # Returns
/// A Result containing the AddressEntry if found, or a DbError if not found or an error occurs
#[allow(dead_code)]
pub fn get_address(address_mac: &str) -> Result<AddressEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    let query = addresses.filter(mac_address.eq(address_mac).and(status.eq("ACTIVE")));

    match query
        .select(AddressEntry::as_select())
        .first::<AddressEntry>(&mut *conn)
    {
        Ok(address) => Ok(address),
        Err(diesel::result::Error::NotFound) => Err(enums::DbError::NotFound),
        Err(e) => {
            log::error!("Database-error: {e:?}");
            Err(enums::DbError::InternalError)
        }
    }
}

/// Lists all addresses.
///
/// This function retrieves all active addresses. There is no permission-based filtering.
///
/// # Returns
/// A QueryResult containing a vector of AddressEntry objects
#[allow(dead_code)]
pub fn list_addresses() -> QueryResult<Vec<AddressEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    addresses
        .filter(status.eq("ACTIVE"))
        .select(AddressEntry::as_select())
        .load(&mut *conn)
}

/// Counts the number of addresses that the user has access to.
///
/// This function counts all active addresses and applies permission-based filtering.
/// The count is filtered based on the user's role and project membership.
///
/// # Arguments
/// * `context` - The user context containing information about the user and their permissions
///
/// # Returns
/// A QueryResult containing the count of addresses as an i64
#[allow(dead_code)]
pub fn count_addresses(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    let mut query = addresses.filter(status.eq("ACTIVE")).into_boxed();

    // Apply permission-based filtering
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Force deletes an address from the database.
///
/// This function marks an address as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Arguments
/// * `address_mac` - The MAC-address of the address to delete
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn force_delete_address(address_mac: &str) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::update(addresses.filter(mac_address.eq(address_mac)))
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

/// Deletes an address from the database.
///
/// This function marks an address as deleted after verifying that it exists.
///
/// # Arguments
/// * `address_mac` - The MAC-address of the address to delete
/// * `context` - The user context of the user, who deletes the address
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_address(address_mac: &str, context: &UserContext) -> Result<(), enums::DbError> {
    // Verify the address exists
    get_address(address_mac)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::update(addresses.filter(mac_address.eq(address_mac)))
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

/// Deletes all addresses from the database.
///
/// This function marks all active addresses as deleted without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_all_addresses() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::update(addresses.filter(status.eq("ACTIVE")))
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

    const INTERNAL_IP: Ipv4Addr = Ipv4Addr::new(10, 0, 0, 2);
    const FLOATING_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 1);

    const TEST_CIDR: &str = "192.168.100.0/24";
    const FIRST_TEST_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 100, 2);
    const LAST_TEST_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 100, 254);

    const MAC_1: &str = "02:00:00:00:10:01";
    const MAC_2: &str = "02:00:00:00:10:02";
    const MAC_3: &str = "02:00:00:00:10:03";

    fn hard_delete_address(address_mac: &str) {
        use self::addresses::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(addresses.filter(mac_address.eq(address_mac))).execute(&mut *conn);
    }

    /// Builds a unique internal IP-address for a test-entry out of the last octet of its MAC-address,
    /// because the internal IP-addresses have to be unique within a network.
    fn internal_ip_of(entry_mac_address: &str) -> Ipv4Addr {
        let number = mac_address_to_number(entry_mac_address).unwrap();
        Ipv4Addr::new(10, 0, 0, number as u8)
    }

    /// Builds an AddressEntry for the tests with the given identity and status.
    fn new_entry(
        entry_mac_address: &str,
        entry_network_uuid: &Uuid,
        entry_owner_id: &str,
        entry_project_id: &str,
        entry_status: &str,
    ) -> AddressEntry {
        AddressEntry {
            mac_address: entry_mac_address.to_string(),
            internal_ip: internal_ip_of(entry_mac_address),
            floating_ip: FLOATING_IP,
            network_uuid: *entry_network_uuid,
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
    fn expect_entry(result: Result<AddressEntry, enums::DbError>) -> AddressEntry {
        match result {
            Ok(entry) => entry,
            Err(_) => panic!("address was not found"),
        }
    }

    /// Asserts that a get-result reports a missing entry.
    fn assert_not_found(result: Result<AddressEntry, enums::DbError>) {
        assert!(matches!(result, Err(enums::DbError::NotFound)));
    }

    #[test]
    #[serial]
    fn test_add_get_address() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();

        let entry = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");

        hard_delete_address(MAC_1);

        add_address(entry.clone()).unwrap();
        let retrieved = expect_entry(get_address(MAC_1));

        assert_eq!(retrieved.mac_address, entry.mac_address);
        assert_eq!(retrieved.internal_ip, entry.internal_ip);
        assert_eq!(retrieved.floating_ip, entry.floating_ip);
        assert_eq!(retrieved.network_uuid, entry.network_uuid);
        assert_eq!(retrieved.owner_id, entry.owner_id);
        assert_eq!(retrieved.project_id, entry.project_id);
        assert_eq!(retrieved.status, entry.status);
        assert_eq!(retrieved.created_at, entry.created_at);
        assert_eq!(retrieved.created_by, entry.created_by);
        assert_eq!(retrieved.updated_at, entry.updated_at);
        assert_eq!(retrieved.updated_by, entry.updated_by);
        assert_eq!(retrieved.deleted_at, entry.deleted_at);
        assert_eq!(retrieved.deleted_by, entry.deleted_by);

        hard_delete_address(MAC_1);
    }

    #[test]
    #[serial]
    fn test_add_new_address() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        hard_delete_address(MAC_1);

        add_new_address(MAC_1, &INTERNAL_IP, &FLOATING_IP, &network_uuid1, &context).unwrap();
        let retrieved = expect_entry(get_address(MAC_1));

        assert_eq!(retrieved.mac_address, MAC_1);
        assert_eq!(retrieved.internal_ip, INTERNAL_IP);
        assert_eq!(retrieved.floating_ip, FLOATING_IP);
        assert_eq!(retrieved.network_uuid, network_uuid1);
        assert_eq!(retrieved.owner_id, "test-user");
        assert_eq!(retrieved.project_id, "test-project");
        assert_eq!(retrieved.status, "ACTIVE");

        hard_delete_address(MAC_1);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_mac_address() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();

        let entry = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");

        hard_delete_address(MAC_1);

        add_address(entry.clone()).unwrap();
        // the mac-address is the primary key, so it must be unique
        assert!(add_address(entry).is_err());

        hard_delete_address(MAC_1);
    }

    #[test]
    fn test_mac_address_conversion() {
        assert_eq!(
            mac_address_to_number("02:00:00:00:00:2a"),
            Some(0x02_00_00_00_00_2a)
        );
        assert_eq!(
            number_to_mac_address(0x02_00_00_00_00_2a),
            "02:00:00:00:00:2a"
        );
        assert_eq!(
            number_to_mac_address(0x02_00_00_00_00_ff + 1),
            "02:00:00:00:01:00"
        );
        assert_eq!(number_to_mac_address(LAST_MAC_ADDRESS), "02:ff:ff:ff:ff:ff");

        assert_eq!(mac_address_to_number("02:00:00:00:00"), None);
        assert_eq!(mac_address_to_number("02:00:00:00:00:2"), None);
        assert_eq!(mac_address_to_number("02:00:00:00:00:zz"), None);
    }

    #[test]
    #[serial]
    fn test_reserve_new_address() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // the first address of a new network gets the first internal IP-address
        let (first_mac, first_ip) =
            reserve_new_address(&FLOATING_IP, &network_uuid1, TEST_CIDR, &context)
                .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(first_ip, FIRST_TEST_IP);
        let retrieved = expect_entry(get_address(&first_mac));
        assert_eq!(retrieved.internal_ip, FIRST_TEST_IP);
        assert_eq!(retrieved.floating_ip, FLOATING_IP);
        assert_eq!(retrieved.network_uuid, network_uuid1);

        // the next reservation gets the next higher values, even if the first entry is deleted
        assert!(delete_address(&first_mac, &context).is_ok());
        let (second_mac, second_ip) =
            reserve_new_address(&FLOATING_IP, &network_uuid1, TEST_CIDR, &context)
                .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(
            mac_address_to_number(&second_mac),
            mac_address_to_number(&first_mac).map(|number| number + 1)
        );
        assert_eq!(u32::from(second_ip), u32::from(first_ip) + 1);

        // internal IP-addresses are counted per network, MAC-addresses globally
        let (third_mac, third_ip) =
            reserve_new_address(&FLOATING_IP, &network_uuid2, TEST_CIDR, &context)
                .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(third_ip, FIRST_TEST_IP);
        assert_eq!(
            mac_address_to_number(&third_mac),
            mac_address_to_number(&second_mac).map(|number| number + 1)
        );

        hard_delete_address(&first_mac);
        hard_delete_address(&second_mac);
        hard_delete_address(&third_mac);
    }

    #[test]
    #[serial]
    fn test_reserve_new_address_invalid_cidr() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let result = reserve_new_address(&FLOATING_IP, &network_uuid1, "10.0.0.0/31", &context);
        assert!(matches!(result, Err(enums::DbError::InternalError)));
    }

    #[test]
    #[serial]
    fn test_reserve_new_address_range_of_cidr() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // an existing address below the range of the CIDR doesn't move the start of the range
        let mut below = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");
        below.internal_ip = Ipv4Addr::new(10, 0, 0, 5);
        hard_delete_address(MAC_1);
        add_address(below).unwrap();

        let (first_mac, first_ip) =
            reserve_new_address(&FLOATING_IP, &network_uuid1, "172.16.0.0/30", &context)
                .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(first_ip, Ipv4Addr::new(172, 16, 0, 2));

        // the /30 has only one assignable address, so the next reservation fails
        let result = reserve_new_address(&FLOATING_IP, &network_uuid1, "172.16.0.0/30", &context);
        assert!(matches!(result, Err(enums::DbError::InternalError)));

        hard_delete_address(MAC_1);
        hard_delete_address(&first_mac);
    }

    #[test]
    #[serial]
    fn test_get_highest_internal_ip_numeric_order() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();

        // alphabetically "10.0.0.9" would be higher than "10.0.0.10"
        let mut entry1 = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");
        entry1.internal_ip = Ipv4Addr::new(10, 0, 0, 9);
        let mut entry2 = new_entry(
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        entry2.internal_ip = Ipv4Addr::new(10, 0, 0, 10);

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);

        assert!(matches!(get_highest_internal_ip(&network_uuid1), Ok(None)));

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

        assert!(matches!(
            get_highest_internal_ip(&network_uuid1),
            Ok(Some(ip)) if ip == Ipv4Addr::new(10, 0, 0, 10)
        ));

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_internal_ip() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        let entry1 = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");
        let mut entry2 = new_entry(MAC_2, &network_uuid1, "test-user", "test-project", "ACTIVE");
        entry2.internal_ip = entry1.internal_ip;
        let mut entry3 = new_entry(MAC_3, &network_uuid2, "test-user", "test-project", "ACTIVE");
        entry3.internal_ip = entry1.internal_ip;

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
        hard_delete_address(MAC_3);

        add_address(entry1).unwrap();
        // the internal IP-address must be unique within a network ...
        assert!(add_address(entry2).is_err());
        // ... but can be used again in another network
        assert!(add_address(entry3).is_ok());

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
        hard_delete_address(MAC_3);
    }

    #[test]
    #[serial]
    fn test_reserve_address_retry_on_mac_conflict() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // simulate other requests in another network, which already reserved the first two MAC-addresses
        let base = mac_address_to_number(MAC_1).unwrap();
        let taken_1 = number_to_mac_address(base);
        let taken_2 = number_to_mac_address(base + 1);
        let expected = number_to_mac_address(base + 2);
        for mac in [&taken_1, &taken_2, &expected] {
            hard_delete_address(mac);
        }
        add_address(new_entry(
            &taken_1,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        ))
        .unwrap();
        add_address(new_entry(
            &taken_2,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        ))
        .unwrap();

        let ip_base = u32::from(FIRST_TEST_IP);
        let (reserved_mac, reserved_ip) = reserve_address_from(
            base,
            ip_base,
            u32::from(LAST_TEST_IP),
            &FLOATING_IP,
            &network_uuid1,
            &context,
        )
        .unwrap_or_else(|_| panic!("reservation failed"));
        assert_eq!(reserved_mac, expected);
        // only the MAC-address was in conflict, so the internal IP-address was not increased
        assert_eq!(reserved_ip, FIRST_TEST_IP);
        assert_eq!(
            expect_entry(get_address(&reserved_mac)).owner_id,
            "test-user"
        );

        for mac in [&taken_1, &taken_2, &expected] {
            hard_delete_address(mac);
        }
    }

    #[test]
    #[serial]
    fn test_reserve_address_retry_on_ip_conflict() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // simulate another request, which already reserved the first internal IP-address
        let mut other = new_entry(
            MAC_2,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );
        other.internal_ip = FIRST_TEST_IP;

        let base = mac_address_to_number(MAC_1).unwrap();
        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
        add_address(other).unwrap();

        let ip_base = u32::from(FIRST_TEST_IP);
        let (reserved_mac, reserved_ip) = reserve_address_from(
            base,
            ip_base,
            u32::from(LAST_TEST_IP),
            &FLOATING_IP,
            &network_uuid1,
            &context,
        )
        .unwrap_or_else(|_| panic!("reservation failed"));
        // only the internal IP-address was in conflict, so the MAC-address was not increased
        assert_eq!(reserved_mac, MAC_1);
        assert_eq!(u32::from(reserved_ip), ip_base + 1);

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
    }

    #[test]
    #[serial]
    fn test_reserve_address_range_exhausted() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let result = reserve_address_from(
            LAST_MAC_ADDRESS + 1,
            u32::from(FIRST_TEST_IP),
            u32::from(LAST_TEST_IP),
            &FLOATING_IP,
            &network_uuid1,
            &context,
        );
        assert!(matches!(result, Err(enums::DbError::InternalError)));

        let result = reserve_address_from(
            FIRST_MAC_ADDRESS,
            u32::from(LAST_TEST_IP) + 1,
            u32::from(LAST_TEST_IP),
            &FLOATING_IP,
            &network_uuid1,
            &context,
        );
        assert!(matches!(result, Err(enums::DbError::InternalError)));
    }

    #[test]
    #[serial]
    fn test_get_address_not_found() {
        let _ = init_address_table();

        hard_delete_address(MAC_1);

        assert_not_found(get_address(MAC_1));
    }

    #[test]
    #[serial]
    fn test_list_addresses() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");
        let entry2 = new_entry(
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

        // only the ACTIVE entry is listed
        let entries = list_addresses().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].mac_address, MAC_1);
        assert_eq!(entries[0].internal_ip, internal_ip_of(MAC_1));
        assert_eq!(entries[0].floating_ip, FLOATING_IP);

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
    }

    #[test]
    #[serial]
    fn test_delete_address() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");

        hard_delete_address(MAC_1);

        add_address(entry).unwrap();
        assert!(delete_address(MAC_1, &context).is_ok());

        assert_not_found(get_address(MAC_1));

        hard_delete_address(MAC_1);
    }

    #[test]
    #[serial]
    fn test_force_delete_address() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();

        // the force-delete works without a user context
        let entry = new_entry(
            MAC_1,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );

        hard_delete_address(MAC_1);

        add_address(entry).unwrap();
        assert!(force_delete_address(MAC_1).is_ok());

        assert_not_found(get_address(MAC_1));

        hard_delete_address(MAC_1);
    }

    #[test]
    #[serial]
    fn test_delete_all_addresses() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");
        let entry2 = new_entry(
            MAC_2,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

        assert!(delete_all_addresses().is_ok());

        assert_eq!(list_addresses().unwrap().len(), 0);

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
    }

    #[test]
    #[serial]
    fn test_count_addresses() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry1 = new_entry(MAC_1, &network_uuid1, "test-user", "test-project", "ACTIVE");
        let entry2 = new_entry(MAC_2, &network_uuid1, "test-user", "test-project", "ACTIVE");
        // entries of other owners are not counted
        let entry3 = new_entry(
            MAC_3,
            &network_uuid1,
            "other-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
        hard_delete_address(MAC_3);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        add_address(entry3).unwrap();

        assert_eq!(count_addresses(&context).unwrap(), 2);

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
        hard_delete_address(MAC_3);
    }

    #[test]
    #[serial]
    fn test_addresses_without_permission_filter() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(
            MAC_1,
            &network_uuid1,
            "test-user-42",
            "test_project_1",
            "ACTIVE",
        );
        let entry2 = new_entry(
            MAC_2,
            &network_uuid1,
            "test-user-43",
            "test_project_1",
            "ACTIVE",
        );
        let entry3 = new_entry(
            MAC_3,
            &network_uuid1,
            "test-user-44",
            "test_project_2",
            "ACTIVE",
        );

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
        hard_delete_address(MAC_3);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        add_address(entry3).unwrap();

        // all addresses are listed, independent of owner and project
        let entries = list_addresses().unwrap();
        for mac in [MAC_1, MAC_2, MAC_3] {
            assert!(entries.iter().any(|entry| entry.mac_address == mac));
        }

        // all addresses can be retrieved, independent of owner and project
        for mac in [MAC_1, MAC_2, MAC_3] {
            let retrieved = expect_entry(get_address(mac));
            assert_eq!(retrieved.mac_address, mac);
        }

        // a normal user can delete an address of another project
        let context = new_context("test-user-42", "test_project_1", false, false);
        assert!(delete_address(MAC_3, &context).is_ok());
        assert_not_found(get_address(MAC_3));

        hard_delete_address(MAC_1);
        hard_delete_address(MAC_2);
        hard_delete_address(MAC_3);
    }
}
