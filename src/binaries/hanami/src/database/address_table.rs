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
use torii_common::VNI_MAX;

/// Prefix of all generated MAC-addresses. The `02` marks them as locally administered
/// unicast addresses, so they can not collide with vendor-assigned addresses.
const MAC_ADDRESS_PREFIX: &str = "02:";

/// Numeric value of the first generated MAC-address `02:00:00:00:00:01`.
const FIRST_MAC_ADDRESS: u64 = 0x02_00_00_00_00_01;

/// Numeric value of the last possible MAC-address `02:ff:ff:ff:ff:ff`.
const LAST_MAC_ADDRESS: u64 = 0x02_ff_ff_ff_ff_ff;

/// Prefix of all generated tap-device names.
const TAP_NAME_PREFIX: &str = "tap-";

/// Number of digits of the counter in the generated tap-device names like `tap-00000001`.
/// With the prefix the names have 12 characters, which is below the limit of 15 characters
/// for network-interface names in Linux.
const TAP_NAME_DIGITS: usize = 8;

/// Counter of the first generated tap-device name `tap-00000001`.
const FIRST_TAP_NUMBER: u32 = 1;

/// Counter of the last possible tap-device name `tap-99999999`.
const LAST_TAP_NUMBER: u32 = 99_999_999;

// Define the schema for addresses table
table! {
    addresses (uuid) {
        uuid -> Varchar,
        mac_address -> Varchar,
        tap_name -> Varchar,
        internal_ip -> Varchar,
        network_uuid -> Varchar,
        vni -> Integer,
        host_address -> Varchar,
        virtual_machine_uuid -> Nullable<Varchar>,
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
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub uuid: Uuid,
    pub mac_address: String,
    pub tap_name: String,
    #[diesel(serialize_as = DbIpv4Addr, deserialize_as = DbIpv4Addr)]
    pub internal_ip: Ipv4Addr,
    #[diesel(serialize_as = DbUuid, deserialize_as = DbUuid)]
    pub network_uuid: Uuid,
    /// Tenant of the address. Every address of one network shares it, because the
    /// virtual_machines of a network have to reach each other, and it is what separates two
    /// networks, which use the same subnet: the torii keys its routes, its packet-filters and
    /// its floating-ip translations by `(vni, address)`. `0` is the shared tenant and belongs
    /// to entries, which were written before the tenants existed.
    #[diesel(serialize_as = DbVni, deserialize_as = DbVni)]
    pub vni: u32,
    /// Address of the sakura-host, which runs the virtual_machine of this address, like
    /// `http://sakura:11420`. The torii, which owns the TAP-device of the virtual_machine, is
    /// derived from it, so the routes of a virtual_machine can be changed later, when another
    /// virtual_machine is added to the same network.
    pub host_address: String,
    /// Virtual_machine, which uses this address. It is set after the virtual_machine was
    /// created on its sakura-host, so it is None for a short time and for entries, which were
    /// written before the virtual_machines were linked with their addresses.
    #[diesel(serialize_as = DbOptUuid, deserialize_as = DbOptUuid)]
    pub virtual_machine_uuid: Option<Uuid>,
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
/// The MAC-address and the tap-device name are unique across all ACTIVE entries and the internal
/// IP-address is unique across all ACTIVE entries within the same network. Deleted entries don't block them.
/// It's typically called during application startup to ensure the required tables exist.
pub fn init_address_table() -> Result<(), Box<dyn Error>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    conn.batch_execute(
        "CREATE TABLE IF NOT EXISTS addresses (
        uuid VARCHAR(40) PRIMARY KEY,
        mac_address VARCHAR(40),
        tap_name VARCHAR(16),
        internal_ip VARCHAR(40),
        network_uuid VARCHAR(40),
        vni INTEGER NOT NULL DEFAULT 0,
        host_address VARCHAR(256),
        virtual_machine_uuid VARCHAR(40),
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
    DROP INDEX IF EXISTS addresses_network_internal_ip;
    CREATE UNIQUE INDEX IF NOT EXISTS addresses_active_mac_address
        ON addresses (mac_address) WHERE status = 'ACTIVE';
    CREATE UNIQUE INDEX IF NOT EXISTS addresses_active_tap_name
        ON addresses (tap_name) WHERE status = 'ACTIVE';
    CREATE UNIQUE INDEX IF NOT EXISTS addresses_active_network_internal_ip
        ON addresses (network_uuid, internal_ip) WHERE status = 'ACTIVE';",
    )?;

    // A database, which was created before the tenants existed, has no vni-column yet, and
    // SQLite has no `ADD COLUMN IF NOT EXISTS`. The error of the second run is therefore the
    // expected outcome and ignored: it only says the column is already there.
    let _ = conn.batch_execute("ALTER TABLE addresses ADD COLUMN vni INTEGER NOT NULL DEFAULT 0;");
    // the same for the link to the virtual_machine, which was added later
    let _ =
        conn.batch_execute("ALTER TABLE addresses ADD COLUMN virtual_machine_uuid VARCHAR(40);");

    Ok(())
}

/// Gets the tenant, which the addresses of a network live in.
///
/// All addresses of a network share one VNI, so the first entry of the network answers the
/// question. Deleted entries are taken into account on purpose: a network keeps its tenant,
/// also while none of its virtual_machines is alive, so an address, which is added later, lands
/// in the same tenant as the floating ip-addresses, which are still registered for it.
///
/// # Arguments
/// * `address_network_uuid` - The UUID of the network
///
/// # Returns
/// A Result containing the VNI of the network, or None if the network never had an address
pub fn get_vni_of_network(address_network_uuid: &Uuid) -> Result<Option<u32>, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    addresses
        .filter(network_uuid.eq(address_network_uuid.to_string()))
        .select(vni)
        .first::<i32>(&mut *conn)
        .optional()
        .map(|found| found.map(|found| found as u32))
        .map_err(|e| {
            log::error!("Database-error: {e:?}");
            enums::DbError::InternalError
        })
}

/// Picks the tenant of a network, assigning a new one if the network has none yet.
///
/// Tenants are handed out per network and start at 1, because `VNI_DEFAULT` (0) is the shared
/// tenant of the uplink and of everything, which was never placed into a tenant of its own.
/// A new tenant is the highest one currently in use increased by one; a tenant of a deleted
/// network is not reused, so no route, which is still lying around somewhere, can be
/// mistaken for a route of the new network.
///
/// # Arguments
/// * `address_network_uuid` - The UUID of the network, which the address belongs to
///
/// # Returns
/// A Result containing the VNI the address has to be created in, or a DbError
fn vni_for_network(address_network_uuid: &Uuid) -> Result<u32, enums::DbError> {
    if let Some(existing) = get_vni_of_network(address_network_uuid)? {
        return Ok(existing);
    }

    let highest = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::addresses::dsl::*;

        addresses
            .select(diesel::dsl::max(vni))
            .first::<Option<i32>>(&mut *conn)
            .map_err(|e| {
                log::error!("Database-error: {e:?}");
                enums::DbError::InternalError
            })?
    };

    let next = highest.unwrap_or(0) as u32 + 1;
    if next > VNI_MAX {
        log::error!("No free VNI left below the 24 bit limit of a VXLAN-header");
        return Err(enums::DbError::InternalError);
    }

    Ok(next)
}

/// Reserves a new MAC-address, a new tap-device name and a new internal IP-address by adding a new entry.
///
/// The new MAC-address and the new tap-device name are the highest ones of all ACTIVE entries
/// increased by one, and the new internal IP-address is the highest internal IP-address of all ACTIVE
/// entries within the same network increased by one. Deleted entries don't block their values, so
/// they can be reused. The reservation is done by inserting the new entry, so none of them can be
/// taken by another request. If another request was faster, the conflicting value is increased again until the
/// reservation was successful.
///
/// # Arguments
/// * `network_uuid` - The UUID of the network the address belongs to
/// * `internal_cidr` - CIDR of the network like `192.168.100.0/24`, which defines the range of the
///   internal IP-addresses. The network-address, the first address (gateway) and the broadcast-address
///   are not assigned.
/// * `host_address` - Address of the sakura-host, which runs the virtual_machine of this address
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the new entry with the reserved values, or a DbError if the reservation failed
#[allow(dead_code)]
pub fn reserve_new_address(
    network_uuid: &Uuid,
    internal_cidr: &str,
    host_address: &str,
    context: &UserContext,
) -> Result<AddressEntry, enums::DbError> {
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

    let first_tap_candidate = match get_highest_tap_number()? {
        Some(highest) => highest + 1,
        None => FIRST_TAP_NUMBER,
    };

    // start behind the highest internal IP-address, but never below the range of the CIDR
    let first_ip_candidate = match get_highest_internal_ip(network_uuid)? {
        Some(highest) => (u32::from(highest) + 1).max(first_internal_ip),
        None => first_internal_ip,
    };

    // every address of a network lives in the same tenant, so the first address of a network
    // is the one, which opens it
    let vni = vni_for_network(network_uuid)?;

    reserve_address_from(
        first_mac_candidate,
        first_tap_candidate,
        first_ip_candidate,
        last_internal_ip,
        network_uuid,
        vni,
        host_address,
        context,
    )
}

/// Tries to reserve MAC-addresses, tap-device names and internal IP-addresses, beginning with the
/// given ones, until the reservation was successful.
///
/// If the insert fails, because the MAC-address, the tap-device name or the internal IP-address was already reserved
/// by another request in the meantime, the conflicting value is increased and the reservation is
/// tried again. If only the UUID was in conflict, a new UUID is generated in the next try.
///
/// # Arguments
/// * `first_mac_candidate` - Numeric value of the first MAC-address to try
/// * `first_tap_candidate` - Counter of the first tap-device name to try
/// * `first_ip_candidate` - Numeric value of the first internal IP-address to try
/// * `last_internal_ip` - Numeric value of the last internal IP-address, which can be assigned
/// * `network_uuid` - The UUID of the network the address belongs to
/// * `vni` - Tenant of the network, which the new address is created in
/// * `host_address` - Address of the sakura-host, which runs the virtual_machine of this address
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the new entry with the reserved values, or a DbError if the reservation failed
#[allow(clippy::too_many_arguments)]
fn reserve_address_from(
    first_mac_candidate: u64,
    first_tap_candidate: u32,
    first_ip_candidate: u32,
    last_internal_ip: u32,
    network_uuid: &Uuid,
    vni: u32,
    host_address: &str,
    context: &UserContext,
) -> Result<AddressEntry, enums::DbError> {
    let mut mac_candidate = first_mac_candidate;
    let mut tap_candidate = first_tap_candidate;
    let mut ip_candidate = first_ip_candidate;
    loop {
        if mac_candidate > LAST_MAC_ADDRESS {
            log::error!(
                "No free MAC-address left in the range of the prefix '{MAC_ADDRESS_PREFIX}'"
            );
            return Err(enums::DbError::InternalError);
        }
        if tap_candidate > LAST_TAP_NUMBER {
            log::error!("No free tap-device name left");
            return Err(enums::DbError::InternalError);
        }
        if ip_candidate > last_internal_ip {
            log::error!("No free internal IP-address left in network '{network_uuid}'");
            return Err(enums::DbError::InternalError);
        }

        let new_mac_address = number_to_mac_address(mac_candidate);
        let new_tap_name = number_to_tap_name(tap_candidate);
        let new_internal_ip = Ipv4Addr::from(ip_candidate);
        match add_new_address(
            &new_mac_address,
            &new_tap_name,
            &new_internal_ip,
            network_uuid,
            vni,
            host_address,
            context,
        ) {
            Ok(address) => return Ok(address),
            // another request was faster and reserved one of the values, so try the next one
            Err(diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, info)) => {
                let message = info.message();
                if message.contains("addresses.mac_address") {
                    mac_candidate += 1;
                }
                if message.contains("addresses.tap_name") {
                    tap_candidate += 1;
                }
                if message.contains("addresses.internal_ip") {
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

/// Gets the highest internal IP-address of all ACTIVE entries within a network from the database.
///
/// Deleted entries are not taken into account, because they don't block their internal
/// IP-address. The IP-addresses are compared numerically, because the alphabetical order
/// of the strings is not the numeric order (`10.0.0.10` < `10.0.0.9`).
///
/// # Arguments
/// * `address_network_uuid` - The UUID of the network
///
/// # Returns
/// A Result containing the highest internal IP-address, or None if the network has no ACTIVE address
fn get_highest_internal_ip(
    address_network_uuid: &Uuid,
) -> Result<Option<Ipv4Addr>, enums::DbError> {
    let internal_ips = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::addresses::dsl::*;

        addresses
            .filter(
                network_uuid
                    .eq(address_network_uuid.to_string())
                    .and(status.eq("ACTIVE")),
            )
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

/// Gets the highest generated MAC-address of all ACTIVE entries from the database.
///
/// Deleted entries are not taken into account, because they don't block their MAC-address.
///
/// # Returns
/// A QueryResult containing the highest MAC-address, or None if there is no ACTIVE generated MAC-address
fn get_highest_mac_address() -> QueryResult<Option<String>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    addresses
        .filter(
            mac_address
                .like(format!("{MAC_ADDRESS_PREFIX}%"))
                .and(status.eq("ACTIVE")),
        )
        .select(mac_address)
        .order(mac_address.desc())
        .first::<String>(&mut *conn)
        .optional()
}

/// Converts a tap-device name like `tap-00000042` into its counter.
///
/// # Returns
/// The counter, or None if the name was not generated in this format
fn tap_name_to_number(name: &str) -> Option<u32> {
    let digits = name.strip_prefix(TAP_NAME_PREFIX)?;
    if digits.len() != TAP_NAME_DIGITS || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    digits.parse::<u32>().ok()
}

/// Converts a counter into a tap-device name like `tap-00000042`.
fn number_to_tap_name(number: u32) -> String {
    format!("{TAP_NAME_PREFIX}{number:0width$}", width = TAP_NAME_DIGITS)
}

/// Gets the counter of the highest generated tap-device name of all ACTIVE entries from the database.
///
/// Deleted entries are not taken into account, because they don't block their tap-device name.
/// Names, which were not generated in the format `tap-00000042`, are ignored.
///
/// # Returns
/// A Result containing the highest counter, or None if there is no ACTIVE generated tap-device name
fn get_highest_tap_number() -> Result<Option<u32>, enums::DbError> {
    let tap_names = {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::addresses::dsl::*;

        addresses
            .filter(
                tap_name
                    .like(format!("{TAP_NAME_PREFIX}%"))
                    .and(status.eq("ACTIVE")),
            )
            .select(tap_name)
            .load::<String>(&mut *conn)
            .map_err(|e| {
                log::error!("Database-error: {e:?}");
                enums::DbError::InternalError
            })?
    };

    Ok(tap_names
        .iter()
        .filter_map(|name| tap_name_to_number(name))
        .max())
}

/// Adds a new address to the database.
///
/// This function creates a new AddressEntry with a new UUID and the provided parameters and inserts
/// it into the database. The status is set to "ACTIVE" and timestamps are set to the current time.
///
/// # Arguments
/// * `mac_address` - The MAC-address of the new entry
/// * `tap_name` - The name of the tap-device of the new entry
/// * `internal_ip` - The internal IP-address assigned to the MAC-address
/// * `network_uuid` - The UUID of the network the address belongs to
/// * `vni` - Tenant of the network, which the new address belongs to
/// * `host_address` - Address of the sakura-host, which runs the virtual_machine of this address
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A QueryResult containing the new entry
#[allow(clippy::too_many_arguments)]
pub fn add_new_address(
    mac_address: &str,
    tap_name: &str,
    internal_ip: &Ipv4Addr,
    network_uuid: &Uuid,
    vni: u32,
    host_address: &str,
    context: &UserContext,
) -> QueryResult<AddressEntry> {
    let address = AddressEntry {
        uuid: Uuid::new_v4(),
        mac_address: mac_address.to_string(),
        tap_name: tap_name.to_string(),
        internal_ip: *internal_ip,
        network_uuid: *network_uuid,
        vni,
        host_address: host_address.to_string(),
        virtual_machine_uuid: None,
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

    add_address(address.clone())?;
    Ok(address)
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
/// This function queries the database for an address with the specified UUID.
/// Only active addresses are returned. There is no permission-based filtering.
///
/// # Arguments
/// * `address_uuid` - The UUID of the address to retrieve
///
/// # Returns
/// A Result containing the AddressEntry if found, or a DbError if not found or an error occurs
#[allow(dead_code)]
pub fn get_address(address_uuid: &Uuid) -> Result<AddressEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    let query = addresses.filter(uuid.eq(address_uuid.to_string()).and(status.eq("ACTIVE")));

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

/// Links an address with the virtual_machine, which uses it.
///
/// The address is reserved before the virtual_machine is created on its sakura-host, so the
/// UUID of the virtual_machine is only known afterwards and set here.
///
/// # Arguments
/// * `address_uuid` - The UUID of the address
/// * `address_virtual_machine_uuid` - The UUID of the virtual_machine, which uses the address
///
/// # Returns
/// A Result indicating success or an error
pub fn set_virtual_machine_of_address(
    address_uuid: &Uuid,
    address_virtual_machine_uuid: &Uuid,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::update(addresses.filter(uuid.eq(address_uuid.to_string())))
        .set((
            virtual_machine_uuid.eq(Some(address_virtual_machine_uuid.to_string())),
            updated_at.eq(Utc::now().to_rfc3339()),
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

/// Retrieves the address of a virtual_machine by the UUID of the virtual_machine.
///
/// Only active addresses are returned. There is no permission-based filtering, so the caller
/// has to verify, that the user is allowed to access the virtual_machine.
///
/// # Arguments
/// * `address_virtual_machine_uuid` - The UUID of the virtual_machine
///
/// # Returns
/// A Result containing the AddressEntry if found, or a DbError if not found or an error occurs
pub fn get_address_of_virtual_machine(
    address_virtual_machine_uuid: &Uuid,
) -> Result<AddressEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    let query = addresses.filter(
        virtual_machine_uuid
            .eq(address_virtual_machine_uuid.to_string())
            .and(status.eq("ACTIVE")),
    );

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

/// Retrieves the address of a virtual_machine by its internal IP-address within a network.
///
/// Only active addresses are returned, because a deleted entry doesn't belong to a running
/// virtual_machine any more. The internal IP-address is unique within a network, so there is at
/// most one entry. There is no permission-based filtering.
///
/// # Arguments
/// * `address_network_uuid` - The UUID of the network of the virtual_machine
/// * `address_internal_ip` - The internal IP-address of the virtual_machine
///
/// # Returns
/// A Result containing the AddressEntry if found, or a DbError if not found or an error occurs
pub fn get_address_by_internal_ip(
    address_network_uuid: &Uuid,
    address_internal_ip: &Ipv4Addr,
) -> Result<AddressEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    let query = addresses.filter(
        network_uuid
            .eq(address_network_uuid.to_string())
            .and(internal_ip.eq(address_internal_ip.to_string()))
            .and(status.eq("ACTIVE")),
    );

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

/// Lists all active addresses within a network.
///
/// The entries contain the internal IP-address and the address of the sakura-host of every
/// virtual_machine of the network, which is what the routes between them are built from. There
/// is no permission-based filtering, because the routes of a network don't belong to a single
/// user.
///
/// # Arguments
/// * `address_network_uuid` - The UUID of the network
///
/// # Returns
/// A QueryResult containing a vector of AddressEntry objects
pub fn list_addresses_of_network(address_network_uuid: &Uuid) -> QueryResult<Vec<AddressEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    addresses
        .filter(
            network_uuid
                .eq(address_network_uuid.to_string())
                .and(status.eq("ACTIVE")),
        )
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
/// * `address_uuid` - The UUID of the address to delete
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn force_delete_address(address_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::update(addresses.filter(uuid.eq(address_uuid.to_string())))
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
/// * `address_uuid` - The UUID of the address to delete
/// * `context` - The user context of the user, who deletes the address
///
/// # Returns
/// A Result indicating success or an error
#[allow(dead_code)]
pub fn delete_address(address_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // Verify the address exists
    get_address(address_uuid)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::update(addresses.filter(uuid.eq(address_uuid.to_string())))
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

    const TEST_CIDR: &str = "192.168.100.0/24";
    const FIRST_TEST_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 100, 2);
    const LAST_TEST_IP: Ipv4Addr = Ipv4Addr::new(192, 168, 100, 254);

    /// Address of the sakura-host of the test-entries, which runs their virtual_machines.
    const TEST_HOST_ADDRESS: &str = "http://sakura:11420";

    /// Tenant the entries of the tests live in.
    const TEST_VNI: u32 = 1;

    const MAC_1: &str = "02:00:00:00:10:01";
    const MAC_2: &str = "02:00:00:00:10:02";
    const MAC_3: &str = "02:00:00:00:10:03";

    /// Counter of the tap-device names in the retry-tests, which is not used by other test-entries.
    const TEST_TAP_BASE: u32 = 90_000_000;

    fn hard_delete_address(address_uuid: &Uuid) {
        use self::addresses::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ =
            diesel::delete(addresses.filter(uuid.eq(address_uuid.to_string()))).execute(&mut *conn);
    }

    /// Removes all entries with the given MAC-address, so leftovers of an aborted
    /// test-run don't block the MAC-addresses of the tests.
    fn hard_delete_mac_address(address_mac: &str) {
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

    /// Builds a unique tap-device name for a test-entry out of the last two octets of its MAC-address,
    /// because the tap-device names have to be unique across the ACTIVE entries.
    fn tap_name_of(entry_mac_address: &str) -> String {
        let number = mac_address_to_number(entry_mac_address).unwrap();
        number_to_tap_name((number & 0xffff) as u32)
    }

    /// Removes all entries with the given tap-device name, so leftovers of an aborted
    /// test-run don't block the tap-device names of the tests.
    fn hard_delete_tap_name(name: &str) {
        use self::addresses::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        let _ = diesel::delete(addresses.filter(tap_name.eq(name))).execute(&mut *conn);
    }

    /// Builds an AddressEntry for the tests with the given identity and status.
    fn new_entry(
        entry_uuid: &Uuid,
        entry_mac_address: &str,
        entry_network_uuid: &Uuid,
        entry_owner_id: &str,
        entry_project_id: &str,
        entry_status: &str,
    ) -> AddressEntry {
        AddressEntry {
            uuid: *entry_uuid,
            mac_address: entry_mac_address.to_string(),
            tap_name: tap_name_of(entry_mac_address),
            internal_ip: internal_ip_of(entry_mac_address),
            network_uuid: *entry_network_uuid,
            vni: TEST_VNI,
            host_address: TEST_HOST_ADDRESS.to_string(),
            virtual_machine_uuid: None,
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

    /// Reads an entry independent of its status.
    fn get_entry_of_any_status(address_uuid: &Uuid) -> AddressEntry {
        use self::addresses::dsl::*;
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        addresses
            .filter(uuid.eq(address_uuid.to_string()))
            .select(AddressEntry::as_select())
            .first::<AddressEntry>(&mut *conn)
            .unwrap()
    }

    #[test]
    #[serial]
    fn test_add_get_address() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);

        add_address(entry.clone()).unwrap();
        let retrieved = expect_entry(get_address(&uuid1));

        assert_eq!(retrieved.uuid, entry.uuid);
        assert_eq!(retrieved.mac_address, entry.mac_address);
        assert_eq!(retrieved.tap_name, entry.tap_name);
        assert_eq!(retrieved.internal_ip, entry.internal_ip);
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

        hard_delete_address(&uuid1);
    }

    #[test]
    #[serial]
    fn test_add_new_address() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let tap_name = "tap-test-1";
        hard_delete_mac_address(MAC_1);
        hard_delete_tap_name(tap_name);

        let added = add_new_address(
            MAC_1,
            tap_name,
            &INTERNAL_IP,
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        )
        .unwrap();
        let uuid1 = added.uuid;
        let retrieved = expect_entry(get_address(&uuid1));

        assert_eq!(retrieved, added);
        assert_eq!(retrieved.mac_address, MAC_1);
        assert_eq!(retrieved.tap_name, tap_name);
        assert_eq!(retrieved.internal_ip, INTERNAL_IP);
        assert_eq!(retrieved.network_uuid, network_uuid1);
        assert_eq!(retrieved.host_address, TEST_HOST_ADDRESS);
        assert_eq!(retrieved.owner_id, "test-user");
        assert_eq!(retrieved.project_id, "test-project");
        assert_eq!(retrieved.status, "ACTIVE");

        hard_delete_address(&uuid1);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_mac_address() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let uuid4 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let deleted1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        let deleted2 = new_entry(
            &uuid2,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        let active1 = new_entry(
            &uuid3,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        // other network, so only the MAC-address is in conflict
        let active2 = new_entry(
            &uuid4,
            MAC_1,
            &network_uuid2,
            "test-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);

        // deleted entries can share a MAC-address with each other and with one ACTIVE entry
        add_address(deleted1).unwrap();
        add_address(deleted2).unwrap();
        add_address(active1).unwrap();
        // but not with a second ACTIVE entry
        assert!(add_address(active2.clone()).is_err());

        // after deleting the ACTIVE entry, the MAC-address can be used again
        assert!(delete_address(&uuid3, &context).is_ok());
        add_address(active2).unwrap();

        hard_delete_mac_address(MAC_1);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_tap_name() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let active1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        // other MAC-address and internal IP-address, so only the tap-device name is in conflict
        let mut active2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        active2.tap_name = active1.tap_name.clone();
        let mut deleted = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        deleted.tap_name = active1.tap_name.clone();

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);
        hard_delete_tap_name(&active1.tap_name);

        add_address(active1).unwrap();
        // the tap-device name must be unique across the ACTIVE entries ...
        assert!(add_address(active2.clone()).is_err());
        // ... but can be shared with deleted entries
        add_address(deleted).unwrap();

        // after deleting the ACTIVE entry, the tap-device name can be used again
        assert!(delete_address(&uuid1, &context).is_ok());
        add_address(active2).unwrap();

        for entry_uuid in [&uuid1, &uuid2, &uuid3] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    fn test_tap_name_conversion() {
        assert_eq!(tap_name_to_number("tap-00000042"), Some(42));
        assert_eq!(number_to_tap_name(42), "tap-00000042");
        assert_eq!(number_to_tap_name(LAST_TAP_NUMBER), "tap-99999999");
        assert!(number_to_tap_name(LAST_TAP_NUMBER).len() <= 15);

        assert_eq!(tap_name_to_number("tap-vm"), None);
        assert_eq!(tap_name_to_number("tap-0000042"), None);
        assert_eq!(tap_name_to_number("tap-+0000042"), None);
        assert_eq!(tap_name_to_number("tun-00000042"), None);
    }

    #[test]
    #[serial]
    fn test_get_highest_tap_number() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        // the highest possible values, so no other entries of the table are higher
        let mut active = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        active.tap_name = number_to_tap_name(LAST_TAP_NUMBER - 10);
        // deleted, so it is ignored
        let mut deleted = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        deleted.tap_name = number_to_tap_name(LAST_TAP_NUMBER - 1);
        // not generated, so it is ignored
        let mut other_format = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        other_format.tap_name = "tap-vm".to_string();

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);
        for name in [&active.tap_name, &deleted.tap_name, &other_format.tap_name] {
            hard_delete_tap_name(name);
        }

        add_address(active).unwrap();
        add_address(deleted).unwrap();
        add_address(other_format).unwrap();

        assert!(matches!(
            get_highest_tap_number(),
            Ok(Some(number)) if number == LAST_TAP_NUMBER - 10
        ));

        for entry_uuid in [&uuid1, &uuid2, &uuid3] {
            hard_delete_address(entry_uuid);
        }
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
        let first = expect_entry(reserve_new_address(
            &network_uuid1,
            TEST_CIDR,
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_eq!(first.internal_ip, FIRST_TEST_IP);
        assert!(tap_name_to_number(&first.tap_name).is_some());
        let retrieved = expect_entry(get_address(&first.uuid));
        assert_eq!(retrieved, first);
        assert_eq!(retrieved.network_uuid, network_uuid1);
        assert_eq!(retrieved.owner_id, "test-user");

        // the next reservation gets the next higher values
        let second = expect_entry(reserve_new_address(
            &network_uuid1,
            TEST_CIDR,
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_ne!(second.uuid, first.uuid);
        assert_eq!(
            mac_address_to_number(&second.mac_address),
            mac_address_to_number(&first.mac_address).map(|number| number + 1)
        );
        assert_eq!(
            tap_name_to_number(&second.tap_name),
            tap_name_to_number(&first.tap_name).map(|number| number + 1)
        );
        assert_eq!(
            u32::from(second.internal_ip),
            u32::from(first.internal_ip) + 1
        );

        // deleted entries don't block their values, so the MAC-address, the tap-device name
        // and the internal IP-address are reused within a new entry
        assert!(delete_address(&second.uuid, &context).is_ok());
        let reused = expect_entry(reserve_new_address(
            &network_uuid1,
            TEST_CIDR,
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_ne!(reused.uuid, second.uuid);
        assert_eq!(reused.mac_address, second.mac_address);
        assert_eq!(reused.tap_name, second.tap_name);
        assert_eq!(reused.internal_ip, second.internal_ip);

        // internal IP-addresses are counted per network, MAC-addresses and tap-device names globally
        let third = expect_entry(reserve_new_address(
            &network_uuid2,
            TEST_CIDR,
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_eq!(third.internal_ip, FIRST_TEST_IP);
        assert_eq!(
            mac_address_to_number(&third.mac_address),
            mac_address_to_number(&reused.mac_address).map(|number| number + 1)
        );
        assert_eq!(
            tap_name_to_number(&third.tap_name),
            tap_name_to_number(&reused.tap_name).map(|number| number + 1)
        );

        for entry_uuid in [&first.uuid, &second.uuid, &reused.uuid, &third.uuid] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_reserve_new_address_invalid_cidr() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let result =
            reserve_new_address(&network_uuid1, "10.0.0.0/31", TEST_HOST_ADDRESS, &context);
        assert!(matches!(result, Err(enums::DbError::InternalError)));
    }

    #[test]
    #[serial]
    fn test_reserve_new_address_range_of_cidr() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // an existing address below the range of the CIDR doesn't move the start of the range
        let mut below = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        below.internal_ip = Ipv4Addr::new(10, 0, 0, 5);
        hard_delete_mac_address(MAC_1);
        add_address(below).unwrap();

        let first = expect_entry(reserve_new_address(
            &network_uuid1,
            "172.16.0.0/30",
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_eq!(first.internal_ip, Ipv4Addr::new(172, 16, 0, 2));

        // the /30 has only one assignable address, so the next reservation fails
        let result =
            reserve_new_address(&network_uuid1, "172.16.0.0/30", TEST_HOST_ADDRESS, &context);
        assert!(matches!(result, Err(enums::DbError::InternalError)));

        hard_delete_address(&uuid1);
        hard_delete_address(&first.uuid);
    }

    #[test]
    #[serial]
    fn test_get_highest_internal_ip() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        // alphabetically "10.0.0.9" would be higher than "10.0.0.10"
        let mut entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry1.internal_ip = Ipv4Addr::new(10, 0, 0, 9);
        let mut entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry2.internal_ip = Ipv4Addr::new(10, 0, 0, 10);
        // deleted, so it is ignored
        let mut entry3 = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        entry3.internal_ip = Ipv4Addr::new(10, 0, 0, 20);

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        assert!(matches!(get_highest_internal_ip(&network_uuid1), Ok(None)));

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        add_address(entry3).unwrap();

        assert!(matches!(
            get_highest_internal_ip(&network_uuid1),
            Ok(Some(ip)) if ip == Ipv4Addr::new(10, 0, 0, 10)
        ));

        for entry_uuid in [&uuid1, &uuid2, &uuid3] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_get_highest_mac_address() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        // the highest possible values, so no other entries of the table are higher
        let active_mac = "02:ff:ff:ff:ff:f0";
        let deleted_mac = "02:ff:ff:ff:ff:fe";
        let active = new_entry(
            &uuid1,
            active_mac,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        // deleted, so it is ignored
        let deleted = new_entry(
            &uuid2,
            deleted_mac,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );

        hard_delete_mac_address(active_mac);
        hard_delete_mac_address(deleted_mac);

        add_address(active).unwrap();
        add_address(deleted).unwrap();

        assert_eq!(
            get_highest_mac_address().unwrap().as_deref(),
            Some(active_mac)
        );

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_internal_ip() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let uuid4 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        let entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let mut entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry2.internal_ip = entry1.internal_ip;
        let mut entry3 = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid2,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry3.internal_ip = entry1.internal_ip;
        let mut entry4 = new_entry(
            &uuid4,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        entry4.internal_ip = entry1.internal_ip;

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        add_address(entry1).unwrap();
        // the internal IP-address must be unique across the ACTIVE entries within a network ...
        assert!(add_address(entry2).is_err());
        // ... but can be used again in another network
        assert!(add_address(entry3).is_ok());
        // ... and by deleted entries
        assert!(add_address(entry4).is_ok());

        for entry_uuid in [&uuid1, &uuid2, &uuid3, &uuid4] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_reserve_address_retry_on_mac_conflict() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // simulate other requests in another network, which already reserved the first two MAC-addresses
        let base = mac_address_to_number(MAC_1).unwrap();
        let taken_1 = number_to_mac_address(base);
        let taken_2 = number_to_mac_address(base + 1);
        let expected = number_to_mac_address(base + 2);
        for mac in [&taken_1, &taken_2, &expected] {
            hard_delete_mac_address(mac);
        }
        add_address(new_entry(
            &uuid1,
            &taken_1,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        ))
        .unwrap();
        add_address(new_entry(
            &uuid2,
            &taken_2,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        ))
        .unwrap();

        let ip_base = u32::from(FIRST_TEST_IP);
        hard_delete_tap_name(&number_to_tap_name(TEST_TAP_BASE));
        let reserved = expect_entry(reserve_address_from(
            base,
            TEST_TAP_BASE,
            ip_base,
            u32::from(LAST_TEST_IP),
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_eq!(reserved.mac_address, expected);
        // only the MAC-address was in conflict, so the other values were not increased
        assert_eq!(reserved.tap_name, number_to_tap_name(TEST_TAP_BASE));
        assert_eq!(reserved.internal_ip, FIRST_TEST_IP);
        assert_eq!(
            expect_entry(get_address(&reserved.uuid)).owner_id,
            "test-user"
        );

        for entry_uuid in [&uuid1, &uuid2, &reserved.uuid] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_reserve_address_retry_on_ip_conflict() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // simulate another request, which already reserved the first internal IP-address
        let mut other = new_entry(
            &uuid1,
            MAC_2,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );
        other.internal_ip = FIRST_TEST_IP;

        let base = mac_address_to_number(MAC_1).unwrap();
        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        add_address(other).unwrap();

        let ip_base = u32::from(FIRST_TEST_IP);
        hard_delete_tap_name(&number_to_tap_name(TEST_TAP_BASE));
        let reserved = expect_entry(reserve_address_from(
            base,
            TEST_TAP_BASE,
            ip_base,
            u32::from(LAST_TEST_IP),
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        ));
        // only the internal IP-address was in conflict, so the other values were not increased
        assert_eq!(reserved.mac_address, MAC_1);
        assert_eq!(reserved.tap_name, number_to_tap_name(TEST_TAP_BASE));
        assert_eq!(u32::from(reserved.internal_ip), ip_base + 1);

        hard_delete_address(&uuid1);
        hard_delete_address(&reserved.uuid);
    }

    #[test]
    #[serial]
    fn test_reserve_address_retry_on_tap_conflict() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        // simulate other requests in another network, which already reserved the first two tap-device names
        let mut taken1 = new_entry(
            &uuid1,
            MAC_2,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        );
        taken1.tap_name = number_to_tap_name(TEST_TAP_BASE);
        let mut taken2 = new_entry(
            &uuid2,
            MAC_3,
            &network_uuid2,
            "other-user",
            "other-project",
            "ACTIVE",
        );
        taken2.tap_name = number_to_tap_name(TEST_TAP_BASE + 1);

        let base = mac_address_to_number(MAC_1).unwrap();
        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);
        for number in TEST_TAP_BASE..=TEST_TAP_BASE + 2 {
            hard_delete_tap_name(&number_to_tap_name(number));
        }
        add_address(taken1).unwrap();
        add_address(taken2).unwrap();

        let reserved = expect_entry(reserve_address_from(
            base,
            TEST_TAP_BASE,
            u32::from(FIRST_TEST_IP),
            u32::from(LAST_TEST_IP),
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        ));
        // only the tap-device name was in conflict, so the other values were not increased
        assert_eq!(reserved.tap_name, number_to_tap_name(TEST_TAP_BASE + 2));
        assert_eq!(reserved.mac_address, MAC_1);
        assert_eq!(reserved.internal_ip, FIRST_TEST_IP);

        for entry_uuid in [&uuid1, &uuid2, &reserved.uuid] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_reserve_address_range_exhausted() {
        let _ = init_address_table();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let result = reserve_address_from(
            LAST_MAC_ADDRESS + 1,
            FIRST_TAP_NUMBER,
            u32::from(FIRST_TEST_IP),
            u32::from(LAST_TEST_IP),
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        );
        assert!(matches!(result, Err(enums::DbError::InternalError)));

        let result = reserve_address_from(
            FIRST_MAC_ADDRESS,
            LAST_TAP_NUMBER + 1,
            u32::from(FIRST_TEST_IP),
            u32::from(LAST_TEST_IP),
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        );
        assert!(matches!(result, Err(enums::DbError::InternalError)));

        let result = reserve_address_from(
            FIRST_MAC_ADDRESS,
            FIRST_TAP_NUMBER,
            u32::from(LAST_TEST_IP) + 1,
            u32::from(LAST_TEST_IP),
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        );
        assert!(matches!(result, Err(enums::DbError::InternalError)));
    }

    #[test]
    #[serial]
    fn test_get_address_not_found() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();

        assert_not_found(get_address(&uuid1));
    }

    #[test]
    #[serial]
    fn test_list_addresses() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

        // only the ACTIVE entry is listed
        let entries: Vec<AddressEntry> = list_addresses()
            .unwrap()
            .into_iter()
            .filter(|entry| entry.network_uuid == network_uuid1)
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, uuid1);
        assert_eq!(entries[0].mac_address, MAC_1);
        assert_eq!(entries[0].internal_ip, internal_ip_of(MAC_1));

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_get_address_by_internal_ip() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        let entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        // the same internal address within another network
        let mut entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid2,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry2.internal_ip = internal_ip_of(MAC_1);

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

        let found = expect_entry(get_address_by_internal_ip(
            &network_uuid1,
            &internal_ip_of(MAC_1),
        ));
        assert_eq!(found.uuid, uuid1);
        assert_eq!(found.host_address, TEST_HOST_ADDRESS);

        // an address, which is not in the network, is not found
        assert_not_found(get_address_by_internal_ip(
            &network_uuid1,
            &internal_ip_of(MAC_3),
        ));

        // a deleted address doesn't belong to a running virtual_machine any more
        let context = new_context("test-user", "test-project", false, false);
        assert!(delete_address(&uuid1, &context).is_ok());
        assert_not_found(get_address_by_internal_ip(
            &network_uuid1,
            &internal_ip_of(MAC_1),
        ));

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_list_addresses_of_network() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        // the virtual_machine of the second host of the same network ...
        let mut entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        entry1.host_address = "http://sakura-2:11420".to_string();
        // ... a deleted one, which has no routes any more ...
        let entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "DELETED",
        );
        // ... and one of another network, which is not connected to the first one
        let entry3 = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid2,
            "test-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        add_address(entry3).unwrap();

        let entries = list_addresses_of_network(&network_uuid1).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, uuid1);
        assert_eq!(entries[0].internal_ip, internal_ip_of(MAC_1));
        // the host-address is required to reach the torii, which owns the TAP-device of the
        // virtual_machine of this address
        assert_eq!(entries[0].host_address, "http://sakura-2:11420");

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
        hard_delete_address(&uuid3);
    }

    #[test]
    #[serial]
    fn test_delete_address() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);

        add_address(entry).unwrap();
        assert!(delete_address(&uuid1, &context).is_ok());
        assert_not_found(get_address(&uuid1));

        // the entry is only marked as deleted
        let deleted = get_entry_of_any_status(&uuid1);
        assert_eq!(deleted.status, "DELETED");
        assert!(deleted.deleted_at.is_some());
        assert_eq!(deleted.deleted_by.as_deref(), Some("test-user"));

        // deleting it again fails
        assert!(matches!(
            delete_address(&uuid1, &context),
            Err(enums::DbError::NotFound)
        ));

        // the MAC-address and the internal IP-address can be used again by a new entry
        let entry = new_entry(
            &uuid2,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        add_address(entry).unwrap();

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_force_delete_address() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        // the force-delete works without a user context
        let entry = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);

        add_address(entry).unwrap();
        assert!(force_delete_address(&uuid1).is_ok());

        assert_not_found(get_address(&uuid1));
        assert_eq!(get_entry_of_any_status(&uuid1).status, "DELETED");

        hard_delete_address(&uuid1);
    }

    #[test]
    #[serial]
    fn test_delete_all_addresses() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "other-user",
            "other-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

        assert!(delete_all_addresses().is_ok());

        assert_eq!(list_addresses().unwrap().len(), 0);
        assert_eq!(get_entry_of_any_status(&uuid1).status, "DELETED");
        assert_eq!(get_entry_of_any_status(&uuid2).status, "DELETED");

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_count_addresses() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );
        // entries of other owners are not counted
        let entry3 = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid1,
            "other-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        add_address(entry3).unwrap();

        assert_eq!(count_addresses(&context).unwrap(), 2);

        for entry_uuid in [&uuid1, &uuid2, &uuid3] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_addresses_without_permission_filter() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user-42",
            "test_project_1",
            "ACTIVE",
        );
        let entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user-43",
            "test_project_1",
            "ACTIVE",
        );
        let entry3 = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid1,
            "test-user-44",
            "test_project_2",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        add_address(entry3).unwrap();

        // all addresses are listed, independent of owner and project
        let entries = list_addresses().unwrap();
        for entry_uuid in [uuid1, uuid2, uuid3] {
            assert!(entries.iter().any(|entry| entry.uuid == entry_uuid));
        }

        // all addresses can be retrieved, independent of owner and project
        for entry_uuid in [uuid1, uuid2, uuid3] {
            let retrieved = expect_entry(get_address(&entry_uuid));
            assert_eq!(retrieved.uuid, entry_uuid);
        }

        // a normal user can delete an address of another project
        let context = new_context("test-user-42", "test_project_1", false, false);
        assert!(delete_address(&uuid3, &context).is_ok());
        assert_not_found(get_address(&uuid3));

        for entry_uuid in [&uuid1, &uuid2, &uuid3] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_virtual_machine_of_address() {
        let _ = init_address_table();
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let virtual_machine_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, false);

        let entry = new_entry(
            &uuid1,
            MAC_1,
            &network_uuid1,
            "test-user",
            "test-project",
            "ACTIVE",
        );

        hard_delete_mac_address(MAC_1);
        add_address(entry).unwrap();

        // the address is not linked with the virtual_machine yet
        assert_not_found(get_address_of_virtual_machine(&virtual_machine_uuid1));

        assert!(set_virtual_machine_of_address(&uuid1, &virtual_machine_uuid1).is_ok());
        let retrieved = expect_entry(get_address_of_virtual_machine(&virtual_machine_uuid1));
        assert_eq!(retrieved.uuid, uuid1);
        assert_eq!(retrieved.virtual_machine_uuid, Some(virtual_machine_uuid1));

        // a deleted address doesn't belong to a virtual_machine any more
        assert!(delete_address(&uuid1, &context).is_ok());
        assert_not_found(get_address_of_virtual_machine(&virtual_machine_uuid1));

        hard_delete_address(&uuid1);
    }
}
