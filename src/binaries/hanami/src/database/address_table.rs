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
use diesel::result::DatabaseErrorKind;
use std::net::Ipv4Addr;
use uuid::Uuid;

use crate::database::{assignable_ip_range, db_handle, is_unique_violation_of, network_table};

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

/// Number of random MAC-addresses, which are checked for a new address, before the reservation
/// gives up.
const MAC_ADDRESS_ATTEMPTS: u32 = 100;

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

/// Number of random tap-device names, which are checked for a new address, before the
/// reservation gives up.
const TAP_NUMBER_ATTEMPTS: u32 = 100;

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
        created_at -> Varchar,
        created_by -> Varchar,
        updated_at -> Varchar,
        updated_by -> Varchar,
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
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    #[diesel(serialize_as = DbDateTime, deserialize_as = DbDateTime)]
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

/// Reserves a new MAC-address, a new tap-device name and a new internal IP-address by adding a new entry.
///
/// The new MAC-address and the new tap-device name are random ones, which no other entry uses,
/// and the new internal IP-address is the highest internal IP-address of all entries within the
/// same network increased by one. The values of deleted entries can be reused. The reservation
/// is done by inserting the new entry, so none of them can be taken by another request. If another
/// request was faster, the conflicting value is picked again until the reservation was successful.
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
pub fn reserve_new_address(
    network_uuid: &Uuid,
    internal_cidr: &str,
    host_address: &str,
    context: &UserContext,
) -> Result<AddressEntry, enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    let (first_internal_ip, last_internal_ip) = match assignable_ip_range(internal_cidr) {
        Some(range) => range,
        None => {
            log::error!("Invalid or too small CIDR '{internal_cidr}' for internal IP-addresses");
            return Err(enums::DbError::InternalError);
        }
    };

    let first_mac_candidate = find_free_mac_address()?;

    let first_tap_candidate = find_free_tap_number()?;

    // start behind the highest internal IP-address, but never below the range of the CIDR
    let first_ip_candidate = match get_highest_internal_ip(network_uuid)? {
        Some(highest) => (u32::from(highest) + 1).max(first_internal_ip),
        None => first_internal_ip,
    };

    // every address of a network lives in the tenant of the network, which the first address
    // of the network assigns
    let vni = network_table::get_or_assign_vni(network_uuid)?;

    reserve_address_from(
        first_mac_candidate,
        first_tap_candidate,
        first_ip_candidate,
        last_internal_ip,
        PredefinedValues::default(),
        network_uuid,
        vni,
        host_address,
        context,
    )
}

/// Values of a reservation, which are predefined and must not be replaced by the next free ones.
#[derive(Debug, Default, Clone, Copy)]
struct PredefinedValues {
    /// The MAC-address is predefined
    mac: bool,
    /// The internal IP-address is predefined
    ip: bool,
}

/// Error of the reservation of an address with predefined values.
pub enum PredefinedAddressError {
    /// A predefined value is already used by another address
    Conflict(String),
    /// A predefined value is invalid or outside of the subnet
    InvalidInput(String),
    /// The reservation failed within the database
    Db(enums::DbError),
}

/// Reserves an address within a network, whose internal IP-address and MAC-address can be
/// predefined.
///
/// It is used to restore a virtual_machine from a backup with its original addresses. The values,
/// which are not predefined, and the tap-device name are reserved like by `reserve_new_address`.
///
/// # Arguments
/// * `network_uuid` - The UUID of the network
/// * `internal_cidr` - The subnet of the network in CIDR-notation
/// * `host_address` - Address of the sakura-host, which runs the virtual_machine of this address
/// * `predefined_ip` - The internal IP-address, which must be used, if any
/// * `predefined_mac` - The MAC-address, which must be used, if any
/// * `context` - The user context containing information about the user and project
///
/// # Returns
/// A Result containing the new entry, `Conflict`, if a predefined value is already used, or
/// `InvalidInput`, if a predefined value is invalid
pub fn reserve_predefined_address(
    network_uuid: &Uuid,
    internal_cidr: &str,
    host_address: &str,
    predefined_ip: Option<&Ipv4Addr>,
    predefined_mac: Option<&str>,
    context: &UserContext,
) -> Result<AddressEntry, PredefinedAddressError> {
    if predefined_ip.is_none() && predefined_mac.is_none() {
        return reserve_new_address(network_uuid, internal_cidr, host_address, context)
            .map_err(PredefinedAddressError::Db);
    }
    if context.is_read_only() {
        return Err(PredefinedAddressError::Db(enums::DbError::PermissionDenied));
    }

    // check if ip is in range
    let Some((first_internal_ip, last_internal_ip)) = assignable_ip_range(internal_cidr) else {
        log::error!("Invalid or too small CIDR '{internal_cidr}' for internal IP-addresses");
        return Err(PredefinedAddressError::Db(enums::DbError::InternalError));
    };

    let db_error = |e: diesel::result::Error| {
        log::error!("Database-error: {e:?}");
        PredefinedAddressError::Db(enums::DbError::InternalError)
    };

    // IP-address
    let ip_candidate = match predefined_ip {
        Some(ip) => {
            let number = u32::from(*ip);
            if number < first_internal_ip || number > last_internal_ip {
                return Err(PredefinedAddressError::InvalidInput(format!(
                    "Internal IP-address '{ip}' is not assignable within '{internal_cidr}'."
                )));
            }
            if is_internal_ip_used(network_uuid, ip).map_err(db_error)? {
                return Err(PredefinedAddressError::Conflict(format!(
                    "Internal IP-address '{ip}' is already used in network '{network_uuid}'."
                )));
            }
            number
        }
        None => match get_highest_internal_ip(network_uuid).map_err(PredefinedAddressError::Db)? {
            Some(highest) => (u32::from(highest) + 1).max(first_internal_ip),
            None => first_internal_ip,
        },
    };

    // MAC-address
    let mac_candidate = match predefined_mac {
        Some(mac) => {
            let number = mac_address_to_number(mac)
                .filter(|number| (FIRST_MAC_ADDRESS..=LAST_MAC_ADDRESS).contains(number))
                .ok_or_else(|| {
                    PredefinedAddressError::InvalidInput(format!(
                        "'{mac}' is no valid MAC-address with the prefix '{MAC_ADDRESS_PREFIX}'."
                    ))
                })?;
            if is_mac_address_used(&number_to_mac_address(number)).map_err(db_error)? {
                return Err(PredefinedAddressError::Conflict(format!(
                    "MAC-address '{mac}' is already used."
                )));
            }
            number
        }
        None => find_free_mac_address().map_err(PredefinedAddressError::Db)?,
    };
    let tap_candidate = find_free_tap_number().map_err(PredefinedAddressError::Db)?;
    let vni = network_table::get_or_assign_vni(network_uuid).map_err(PredefinedAddressError::Db)?;

    reserve_address_from(
        mac_candidate,
        tap_candidate,
        ip_candidate,
        // a predefined internal IP-address is the only one, which is tried
        if predefined_ip.is_some() {
            ip_candidate
        } else {
            last_internal_ip
        },
        PredefinedValues {
            mac: predefined_mac.is_some(),
            ip: predefined_ip.is_some(),
        },
        network_uuid,
        vni,
        host_address,
        context,
    )
    .map_err(PredefinedAddressError::Db)
}

/// Checks, if an internal IP-address is used by an active address of a network.
fn is_internal_ip_used(address_network_uuid: &Uuid, ip: &Ipv4Addr) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    addresses
        .filter(network_uuid.eq(address_network_uuid.to_string()))
        .filter(internal_ip.eq(ip.to_string()))
        .select(count_star())
        .first::<i64>(&mut *conn)
        .map(|count| count > 0)
}

/// Checks, if a MAC-address is used by an active address.
fn is_mac_address_used(address_mac: &str) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    addresses
        .filter(mac_address.eq(address_mac))
        .select(count_star())
        .first::<i64>(&mut *conn)
        .map(|count| count > 0)
}

/// Tries to reserve MAC-addresses, tap-device names and internal IP-addresses, beginning with the
/// given ones, until the reservation was successful.
///
/// If the insert fails, because the MAC-address, the tap-device name or the internal IP-address
/// was already reserved by another request in the meantime, the reservation is tried again: a
/// conflicting MAC-address is replaced by a new random one, a conflicting tap-device name or
/// internal IP-address is increased. If only the UUID was in conflict, a new UUID is generated in
/// the next try.
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
    predefined: PredefinedValues,
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
                let mac_taken = is_unique_violation_of(message, "addresses", "mac_address");
                let ip_taken = is_unique_violation_of(message, "addresses", "internal_ip");
                // a predefined value was checked to be free before, so another request was faster
                if (predefined.mac && mac_taken) || (predefined.ip && ip_taken) {
                    log::error!(
                        "Predefined MAC- or internal IP-address was reserved by another request in \
                         the meantime"
                    );
                    return Err(enums::DbError::InternalError);
                }
                // a MAC-address is not counted up, but picked randomly again
                if mac_taken {
                    mac_candidate = find_free_mac_address()?;
                }
                if is_unique_violation_of(message, "addresses", "tap_name") {
                    tap_candidate += 1;
                }
                if ip_taken {
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

/// Gets the highest internal IP-address of all entries within a network from the database.
///
/// The IP-addresses are compared numerically, because the alphabetical order of the strings is not
/// the numeric order (`10.0.0.10` < `10.0.0.9`).
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

/// Picks a random MAC-address with the prefix `02:`, which is not used by another entry.
///
/// Up to `MAC_ADDRESS_ATTEMPTS` random MAC-addresses are checked one after another and the first
/// free one is taken. The MAC-address is only
/// checked, so another request can still take it, before it is inserted, which
/// `reserve_address_from` handles.
///
/// # Returns
/// A Result containing the numeric value of the free MAC-address, or a DbError if none of the
/// checked MAC-addresses was free
fn find_free_mac_address() -> Result<u64, enums::DbError> {
    for _ in 0..MAC_ADDRESS_ATTEMPTS {
        let candidate = rand::random_range(FIRST_MAC_ADDRESS..=LAST_MAC_ADDRESS);
        let used = is_mac_address_used(&number_to_mac_address(candidate)).map_err(|e| {
            log::error!("Database-error: {e:?}");
            enums::DbError::InternalError
        })?;
        if !used {
            return Ok(candidate);
        }
    }

    log::error!("No free MAC-address found within {MAC_ADDRESS_ATTEMPTS} random attempts");
    Err(enums::DbError::InternalError)
}

/// Converts a counter into a tap-device name like `tap-00000042`.
fn number_to_tap_name(number: u32) -> String {
    format!("{TAP_NAME_PREFIX}{number:0width$}", width = TAP_NAME_DIGITS)
}

/// Picks a random tap-device name, which is not used by another entry.
///
/// Up to `TAP_NUMBER_ATTEMPTS` random counters are checked one after another and the first free
/// one is taken. The name is only checked, so
/// another request can still take it, before it is inserted, which `reserve_address_from` handles.
///
/// # Returns
/// A Result containing the counter of the free tap-device name, or a DbError if none of the
/// checked names was free
fn find_free_tap_number() -> Result<u32, enums::DbError> {
    for _ in 0..TAP_NUMBER_ATTEMPTS {
        let candidate = rand::random_range(FIRST_TAP_NUMBER..=LAST_TAP_NUMBER);
        let used = is_tap_name_used(&number_to_tap_name(candidate)).map_err(|e| {
            log::error!("Database-error: {e:?}");
            enums::DbError::InternalError
        })?;
        if !used {
            return Ok(candidate);
        }
    }

    log::error!("No free tap-device name found within {TAP_NUMBER_ATTEMPTS} random attempts");
    Err(enums::DbError::InternalError)
}

/// Checks, if a tap-device name is used by an active address.
fn is_tap_name_used(name: &str) -> QueryResult<bool> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    addresses
        .filter(tap_name.eq(name))
        .select(count_star())
        .first::<i64>(&mut *conn)
        .map(|count| count > 0)
}

/// Adds a new address to the database.
///
/// This function creates a new AddressEntry with a new UUID and the provided parameters and inserts
/// it into the database. The timestamps are set to the current time.
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
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::permission_denied_error());
    }

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
        created_at: Utc::now(),
        created_by: context.user_id.clone(),
        updated_at: Utc::now(),
        updated_by: context.user_id.clone(),
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
pub fn get_address(address_uuid: &Uuid) -> Result<AddressEntry, enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    let query = addresses.filter(uuid.eq(address_uuid.to_string()));

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

/// Moves an address to another sakura-host, after its virtual_machine was migrated there.
///
/// The address of the host is what the routes towards the virtual_machine are created with, so
/// it has to follow the virtual_machine.
///
/// # Arguments
/// * `address_uuid` - The UUID of the address
/// * `new_host_address` - Address of the sakura-host, which runs the virtual_machine now
///
/// # Returns
/// A Result indicating success or an error
pub fn set_host_of_address(
    address_uuid: &Uuid,
    new_host_address: &str,
) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::update(addresses.filter(uuid.eq(address_uuid.to_string())))
        .set((
            host_address.eq(new_host_address),
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

    let query = addresses.filter(virtual_machine_uuid.eq(address_virtual_machine_uuid.to_string()));

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
/// The internal IP-address is unique within a network, so there is at most one entry. There is
/// no permission-based filtering.
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
            .and(internal_ip.eq(address_internal_ip.to_string())),
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
pub fn list_addresses() -> QueryResult<Vec<AddressEntry>> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    addresses.select(AddressEntry::as_select()).load(&mut *conn)
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
        .filter(network_uuid.eq(address_network_uuid.to_string()))
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
#[cfg(test)]
pub fn count_addresses(context: &UserContext) -> QueryResult<i64> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;

    let mut query = addresses.into_boxed();

    // Apply permission-based filtering
    query = query.filter(project_id.eq(context.project_id.clone()));
    query = query.filter(owner_id.eq(context.user_id.clone()));

    query.select(count_star()).first::<i64>(&mut *conn)
}

/// Force deletes an address from the database.
///
/// This function removes an address without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Arguments
/// * `address_uuid` - The UUID of the address to delete
///
/// # Returns
/// A Result indicating success or an error
pub fn force_delete_address(address_uuid: &Uuid) -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::delete(addresses.filter(uuid.eq(address_uuid.to_string()))).execute(&mut *conn) {
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
/// This function removes an address after verifying that it exists.
///
/// # Arguments
/// * `address_uuid` - The UUID of the address to delete
/// * `context` - The user context of the user, who deletes the address
///
/// # Returns
/// A Result indicating success or an error
pub fn delete_address(address_uuid: &Uuid, context: &UserContext) -> Result<(), enums::DbError> {
    // observers without admin-privileges are only allowed to read
    if context.is_read_only() {
        return Err(enums::DbError::PermissionDenied);
    }

    // Verify the address exists
    get_address(address_uuid)?;

    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::delete(addresses.filter(uuid.eq(address_uuid.to_string()))).execute(&mut *conn) {
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
/// This function removes all addresses without checking permissions.
/// It's intended for system-level operations where permission checks are not required.
///
/// # Returns
/// A Result indicating success or an error
#[cfg(test)]
pub fn delete_all_addresses() -> Result<(), enums::DbError> {
    let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
    use self::addresses::dsl::*;
    match diesel::delete(addresses).execute(&mut *conn) {
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
    /// because the tap-device names have to be unique.
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

    /// Builds an AddressEntry for the tests with the given identity.
    fn new_entry(
        entry_uuid: &Uuid,
        entry_mac_address: &str,
        entry_network_uuid: &Uuid,
        entry_owner_id: &str,
        entry_project_id: &str,
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
            created_at: Utc::now(),
            created_by: "admin".to_string(),
            updated_at: Utc::now(),
            updated_by: "admin".to_string(),
        }
    }

    /// Adds a network, because a reservation takes the tenant of its network. It lives in a
    /// project of its own, so it is not counted by the tests of the networks.
    fn add_test_network(network_uuid: &Uuid) {
        let context = new_context(
            "test-user",
            "test-address-network",
            false,
            ProjectRole::Member,
        );
        network_table::add_new_network(network_uuid, "test-network", TEST_CIDR, false, &context)
            .unwrap();
    }

    fn remove_test_network(network_uuid: &Uuid) {
        let _ = network_table::force_delete_network(network_uuid);
    }

    /// Builds a UserContext for the tests.
    fn new_context(
        user_id: &str,
        project_id: &str,
        is_admin: bool,
        project_role: ProjectRole,
    ) -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: user_id.to_string(),
            project_id: project_id.to_string(),
            is_admin: is_admin.to_string(),
            project_role: project_role.to_string(),
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
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");

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
        assert_eq!(retrieved.created_at, entry.created_at);
        assert_eq!(retrieved.created_by, entry.created_by);
        assert_eq!(retrieved.updated_at, entry.updated_at);
        assert_eq!(retrieved.updated_by, entry.updated_by);

        hard_delete_address(&uuid1);
    }

    #[test]
    #[serial]
    fn test_add_new_address() {
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

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

        hard_delete_address(&uuid1);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_mac_address() {
        let uuid3 = Uuid::new_v4();
        let uuid4 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let active1 = new_entry(&uuid3, MAC_1, &network_uuid1, "test-user", "test-project");
        // other network, so only the MAC-address is in conflict
        let active2 = new_entry(&uuid4, MAC_1, &network_uuid2, "test-user", "test-project");

        hard_delete_mac_address(MAC_1);

        // the MAC-address must be unique ...
        add_address(active1).unwrap();
        assert!(add_address(active2.clone()).is_err());

        // ... and after deleting the entry, it can be used again
        assert!(delete_address(&uuid3, &context).is_ok());
        add_address(active2).unwrap();

        hard_delete_mac_address(MAC_1);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_tap_name() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let active1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        // other MAC-address and internal IP-address, so only the tap-device name is in conflict
        let mut active2 = new_entry(&uuid2, MAC_2, &network_uuid1, "test-user", "test-project");
        active2.tap_name = active1.tap_name.clone();

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);
        hard_delete_tap_name(&active1.tap_name);

        add_address(active1).unwrap();
        // the tap-device name must be unique ...
        assert!(add_address(active2.clone()).is_err());

        // ... and after deleting the entry, it can be used again
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
    fn test_find_free_tap_number() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let mut active = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        active.tap_name = number_to_tap_name(TEST_TAP_BASE);

        hard_delete_mac_address(MAC_1);
        hard_delete_tap_name(&active.tap_name);
        add_address(active.clone()).unwrap();

        // only an existing entry blocks its tap-device name
        assert!(is_tap_name_used(&active.tap_name).unwrap());
        assert!(!is_tap_name_used(&number_to_tap_name(TEST_TAP_BASE + 1)).unwrap());

        // the picked tap-device names are within the range and free
        for _ in 0..20 {
            let number = find_free_tap_number()
                .ok()
                .expect("no free tap-device name found");
            assert!((FIRST_TAP_NUMBER..=LAST_TAP_NUMBER).contains(&number));
            assert!(!is_tap_name_used(&number_to_tap_name(number)).unwrap());
        }

        for entry_uuid in [&uuid1, &uuid2] {
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
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);
        add_test_network(&network_uuid1);
        add_test_network(&network_uuid2);

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
        // the MAC-address and the tap-device name are random free ones
        assert_ne!(second.mac_address, first.mac_address);
        assert!(tap_name_to_number(&second.tap_name).is_some());
        assert_ne!(second.tap_name, first.tap_name);
        assert_eq!(
            u32::from(second.internal_ip),
            u32::from(first.internal_ip) + 1
        );

        // the values of a deleted entry can be used again, so the internal IP-address is reused
        // within a new entry
        assert!(delete_address(&second.uuid, &context).is_ok());
        let reused = expect_entry(reserve_new_address(
            &network_uuid1,
            TEST_CIDR,
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_ne!(reused.uuid, second.uuid);
        assert_ne!(reused.mac_address, first.mac_address);
        assert_ne!(reused.tap_name, first.tap_name);
        assert_eq!(reused.internal_ip, second.internal_ip);

        // internal IP-addresses are counted per network, MAC-addresses and tap-device names are
        // unique globally
        let third = expect_entry(reserve_new_address(
            &network_uuid2,
            TEST_CIDR,
            TEST_HOST_ADDRESS,
            &context,
        ));
        assert_eq!(third.internal_ip, FIRST_TEST_IP);
        assert!(![&first.mac_address, &reused.mac_address].contains(&&third.mac_address));
        assert!(![&first.tap_name, &reused.tap_name].contains(&&third.tap_name));

        for entry_uuid in [&first.uuid, &second.uuid, &reused.uuid, &third.uuid] {
            hard_delete_address(entry_uuid);
        }
        remove_test_network(&network_uuid1);
        remove_test_network(&network_uuid2);
    }

    #[test]
    #[serial]
    fn test_reserve_new_address_invalid_cidr() {
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let result =
            reserve_new_address(&network_uuid1, "10.0.0.0/31", TEST_HOST_ADDRESS, &context);
        assert!(matches!(result, Err(enums::DbError::InternalError)));
    }

    #[test]
    #[serial]
    fn test_reserve_new_address_range_of_cidr() {
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);
        add_test_network(&network_uuid1);

        // an existing address below the range of the CIDR doesn't move the start of the range
        let mut below = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
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
        remove_test_network(&network_uuid1);
    }

    #[test]
    #[serial]
    fn test_get_highest_internal_ip() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        // alphabetically "10.0.0.9" would be higher than "10.0.0.10"
        let mut entry1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        entry1.internal_ip = Ipv4Addr::new(10, 0, 0, 9);
        let mut entry2 = new_entry(&uuid2, MAC_2, &network_uuid1, "test-user", "test-project");
        entry2.internal_ip = Ipv4Addr::new(10, 0, 0, 10);

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        assert!(matches!(get_highest_internal_ip(&network_uuid1), Ok(None)));

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

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
    fn test_find_free_mac_address() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let active = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        add_address(active).unwrap();

        // only an existing entry blocks its MAC-address
        assert!(is_mac_address_used(MAC_1).unwrap());
        assert!(!is_mac_address_used(MAC_2).unwrap());

        // the picked MAC-addresses are within the range of the prefix and free
        for _ in 0..20 {
            let number = find_free_mac_address()
                .ok()
                .expect("no free MAC-address found");
            assert!((FIRST_MAC_ADDRESS..=LAST_MAC_ADDRESS).contains(&number));
            let mac = number_to_mac_address(number);
            assert!(mac.starts_with(MAC_ADDRESS_PREFIX));
            assert!(!is_mac_address_used(&mac).unwrap());
        }

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_add_duplicate_internal_ip() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        let entry1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        let mut entry2 = new_entry(&uuid2, MAC_2, &network_uuid1, "test-user", "test-project");
        entry2.internal_ip = entry1.internal_ip;
        let mut entry3 = new_entry(&uuid3, MAC_3, &network_uuid2, "test-user", "test-project");
        entry3.internal_ip = entry1.internal_ip;

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        add_address(entry1).unwrap();
        // the internal IP-address must be unique within a network ...
        assert!(add_address(entry2).is_err());
        // ... but can be used again in another network
        assert!(add_address(entry3).is_ok());

        for entry_uuid in [&uuid1, &uuid2, &uuid3] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_reserve_address_retry_on_mac_conflict() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        // simulate other requests in another network, which already reserved the first
        // MAC-address to try and the one behind it
        let base = mac_address_to_number(MAC_1).unwrap();
        let taken_1 = number_to_mac_address(base);
        let taken_2 = number_to_mac_address(base + 1);
        for mac in [&taken_1, &taken_2] {
            hard_delete_mac_address(mac);
        }
        add_address(new_entry(
            &uuid1,
            &taken_1,
            &network_uuid2,
            "other-user",
            "other-project",
        ))
        .unwrap();
        add_address(new_entry(
            &uuid2,
            &taken_2,
            &network_uuid2,
            "other-user",
            "other-project",
        ))
        .unwrap();

        let ip_base = u32::from(FIRST_TEST_IP);
        hard_delete_tap_name(&number_to_tap_name(TEST_TAP_BASE));
        let reserved = expect_entry(reserve_address_from(
            base,
            TEST_TAP_BASE,
            ip_base,
            u32::from(LAST_TEST_IP),
            PredefinedValues::default(),
            &network_uuid1,
            TEST_VNI,
            TEST_HOST_ADDRESS,
            &context,
        ));
        // the MAC-address is not counted up, but a new random one is picked
        assert!(![&taken_1, &taken_2].contains(&&reserved.mac_address));
        assert!(reserved.mac_address.starts_with(MAC_ADDRESS_PREFIX));
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
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        // simulate another request, which already reserved the first internal IP-address
        let mut other = new_entry(&uuid1, MAC_2, &network_uuid1, "other-user", "other-project");
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
            PredefinedValues::default(),
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
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        // simulate other requests in another network, which already reserved the first two tap-device names
        let mut taken1 = new_entry(&uuid1, MAC_2, &network_uuid2, "other-user", "other-project");
        taken1.tap_name = number_to_tap_name(TEST_TAP_BASE);
        let mut taken2 = new_entry(&uuid2, MAC_3, &network_uuid2, "other-user", "other-project");
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
            PredefinedValues::default(),
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
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let result = reserve_address_from(
            LAST_MAC_ADDRESS + 1,
            FIRST_TAP_NUMBER,
            u32::from(FIRST_TEST_IP),
            u32::from(LAST_TEST_IP),
            PredefinedValues::default(),
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
            PredefinedValues::default(),
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
            PredefinedValues::default(),
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
        let uuid1 = Uuid::new_v4();

        assert_not_found(get_address(&uuid1));
    }

    #[test]
    #[serial]
    fn test_list_addresses() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        let entry2 = new_entry(&uuid2, MAC_2, &network_uuid1, "test-user", "test-project");

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        assert!(force_delete_address(&uuid2).is_ok());

        // the deleted entry is not listed anymore
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
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        let entry1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        // the same internal address within another network
        let mut entry2 = new_entry(&uuid2, MAC_2, &network_uuid2, "test-user", "test-project");
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
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);
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
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let network_uuid2 = Uuid::new_v4();

        // the virtual_machine of the second host of the same network ...
        let mut entry1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        entry1.host_address = "http://sakura-2:11420".to_string();
        // ... a deleted one, which has no routes any more ...
        let entry2 = new_entry(&uuid2, MAC_2, &network_uuid1, "test-user", "test-project");
        // ... and one of another network, which is not connected to the first one
        let entry3 = new_entry(&uuid3, MAC_3, &network_uuid2, "test-user", "test-project");

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);
        hard_delete_mac_address(MAC_3);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();
        add_address(entry3).unwrap();
        assert!(force_delete_address(&uuid2).is_ok());

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
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let entry = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");

        hard_delete_mac_address(MAC_1);

        add_address(entry).unwrap();
        assert!(delete_address(&uuid1, &context).is_ok());
        assert_not_found(get_address(&uuid1));

        // deleting it again fails
        assert!(matches!(
            delete_address(&uuid1, &context),
            Err(enums::DbError::NotFound)
        ));

        // the MAC-address and the internal IP-address can be used again by a new entry
        let entry = new_entry(&uuid2, MAC_1, &network_uuid1, "test-user", "test-project");
        add_address(entry).unwrap();

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_force_delete_address() {
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        // the force-delete works without a user context
        let entry = new_entry(&uuid1, MAC_1, &network_uuid1, "other-user", "other-project");

        hard_delete_mac_address(MAC_1);

        add_address(entry).unwrap();
        assert!(force_delete_address(&uuid1).is_ok());

        assert_not_found(get_address(&uuid1));

        hard_delete_address(&uuid1);
    }

    #[test]
    #[serial]
    fn test_delete_all_addresses() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();

        let entry1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        let entry2 = new_entry(&uuid2, MAC_2, &network_uuid1, "other-user", "other-project");

        hard_delete_mac_address(MAC_1);
        hard_delete_mac_address(MAC_2);

        add_address(entry1).unwrap();
        add_address(entry2).unwrap();

        assert!(delete_all_addresses().is_ok());

        assert_eq!(list_addresses().unwrap().len(), 0);

        hard_delete_address(&uuid1);
        hard_delete_address(&uuid2);
    }

    #[test]
    #[serial]
    fn test_count_addresses() {
        let uuid1 = Uuid::new_v4();
        let uuid2 = Uuid::new_v4();
        let uuid3 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let entry1 = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");
        let entry2 = new_entry(&uuid2, MAC_2, &network_uuid1, "test-user", "test-project");
        // entries of other owners are not counted
        let entry3 = new_entry(&uuid3, MAC_3, &network_uuid1, "other-user", "test-project");

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
        );
        let entry2 = new_entry(
            &uuid2,
            MAC_2,
            &network_uuid1,
            "test-user-43",
            "test_project_1",
        );
        let entry3 = new_entry(
            &uuid3,
            MAC_3,
            &network_uuid1,
            "test-user-44",
            "test_project_2",
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
        let context = new_context("test-user-42", "test_project_1", false, ProjectRole::Member);
        assert!(delete_address(&uuid3, &context).is_ok());
        assert_not_found(get_address(&uuid3));

        for entry_uuid in [&uuid1, &uuid2, &uuid3] {
            hard_delete_address(entry_uuid);
        }
    }

    #[test]
    #[serial]
    fn test_virtual_machine_of_address() {
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let virtual_machine_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let entry = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");

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

    #[test]
    #[serial]
    fn test_host_of_address() {
        let uuid1 = Uuid::new_v4();
        let network_uuid1 = Uuid::new_v4();
        let context = new_context("test-user", "test-project", false, ProjectRole::Member);

        let entry = new_entry(&uuid1, MAC_1, &network_uuid1, "test-user", "test-project");

        hard_delete_mac_address(MAC_1);
        add_address(entry).unwrap();

        assert!(set_host_of_address(&uuid1, "http://10.0.0.7:11420").is_ok());
        let retrieved = expect_entry(get_address(&uuid1));
        assert_eq!(retrieved.host_address, "http://10.0.0.7:11420");

        // a deleted address is not moved anymore
        assert!(delete_address(&uuid1, &context).is_ok());
        assert!(matches!(
            set_host_of_address(&uuid1, "http://10.0.0.8:11420"),
            Err(enums::DbError::NotFound)
        ));
        assert!(matches!(
            set_host_of_address(&Uuid::new_v4(), "http://10.0.0.8:11420"),
            Err(enums::DbError::NotFound)
        ));

        hard_delete_address(&uuid1);
    }

    fn predefined_context() -> UserContext {
        UserContext {
            token: "".to_string(),
            user_id: "test-user".to_string(),
            project_id: "test-predefined-address".to_string(),
            is_admin: false.to_string(),
            project_role: ainari_common::enums::ProjectRole::Member.to_string(),
        }
    }

    fn hard_delete_network_addresses(network: &Uuid) {
        let mut conn = db_handle::DB_CONN.lock().expect("mutex poisoned");
        use self::addresses::dsl::*;
        let _ = diesel::delete(addresses.filter(network_uuid.eq(network.to_string())))
            .execute(&mut *conn);
    }

    #[test]
    #[serial]
    fn test_reserve_predefined_address() {
        let network = Uuid::new_v4();
        let context = predefined_context();
        add_test_network(&network);
        let cidr = "10.20.0.0/24";
        let ip = Ipv4Addr::new(10, 20, 0, 50);
        let mac = "02:aa:bb:cc:dd:01";

        // both values are kept
        let Ok(address) =
            reserve_predefined_address(&network, cidr, "host", Some(&ip), Some(mac), &context)
        else {
            panic!("reservation with predefined values failed");
        };
        assert_eq!(address.internal_ip, ip);
        assert_eq!(address.mac_address, mac);

        // a used internal IP-address or MAC-address is a conflict
        let other_ip = Ipv4Addr::new(10, 20, 0, 51);
        assert!(matches!(
            reserve_predefined_address(&network, cidr, "host", Some(&ip), None, &context),
            Err(PredefinedAddressError::Conflict(_))
        ));
        assert!(matches!(
            reserve_predefined_address(
                &network,
                cidr,
                "host",
                Some(&other_ip),
                Some(mac),
                &context
            ),
            Err(PredefinedAddressError::Conflict(_))
        ));

        // invalid values
        let outside = Ipv4Addr::new(10, 21, 0, 5);
        assert!(matches!(
            reserve_predefined_address(&network, cidr, "host", Some(&outside), None, &context),
            Err(PredefinedAddressError::InvalidInput(_))
        ));
        for invalid_mac in ["no-mac", "52:54:00:aa:bb:01"] {
            assert!(matches!(
                reserve_predefined_address(
                    &network,
                    cidr,
                    "host",
                    None,
                    Some(invalid_mac),
                    &context
                ),
                Err(PredefinedAddressError::InvalidInput(_))
            ));
        }

        // only the internal IP-address is predefined, the MAC-address is the next free one
        let Ok(address) =
            reserve_predefined_address(&network, cidr, "host", Some(&other_ip), None, &context)
        else {
            panic!("reservation with predefined internal IP-address failed");
        };
        assert_eq!(address.internal_ip, other_ip);
        assert_ne!(address.mac_address, mac);

        hard_delete_network_addresses(&network);
        remove_test_network(&network);
    }
}
