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

pub mod address_table;
pub mod db_handle;
pub mod floating_ip_table;
pub mod host_table;
pub mod meta_virtual_machine_table;
pub mod network_table;
pub mod vm_type_table;

use std::net::Ipv4Addr;

/// Opens the database of the service and applies all pending migrations of the
/// `migrations`-directory, which create and update the database-tables.
///
/// # Panics
///
/// Panics, if the database can not be opened or a migration fails, because the service can not
/// work with an incomplete database.
///
/// # Returns
///
/// * `Ok(())` - The database is up to date.
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Open the database and apply all pending migrations, which creates and updates the tables.
    // This is done explicitly here, so a broken database is already detected at startup.
    lazy_static::initialize(&db_handle::DB_CONN);
    log::info!("Applied all database-migrations");
    Ok(())
}

/// Checks, if a unique-violation was caused by the unique index of the given column.
///
/// The databases describe the violation differently in their error-message: sqlite lists the
/// columns of the index like `UNIQUE constraint failed: addresses.mac_address`, while mysql names
/// the index like `Duplicate entry '...' for key 'addresses.addresses_active_mac_address'`. The
/// unique indexes of the mysql-migrations are therefore named after the column, which they make
/// unique, at their end.
///
/// # Arguments
/// * `message` - Error-message of the unique-violation
/// * `table` - Name of the table
/// * `column` - Name of the column
///
/// # Returns
/// True, if the unique index of the column was violated
pub fn is_unique_violation_of(message: &str, table: &str, column: &str) -> bool {
    match message.split_once(" for key ") {
        Some((_, key)) => key.trim_matches('\'').ends_with(column),
        None => message.contains(&format!("{table}.{column}")),
    }
}

/// Calculates the range of the assignable IP-addresses of a CIDR like `192.168.100.0/24`.
///
/// The network-address and the first address, which is reserved for the gateway, are skipped at
/// the beginning, and the broadcast-address at the end.
///
/// # Returns
/// The numeric values of the first and last assignable IP-address, or None if the CIDR is invalid
/// or too small to contain at least one assignable IP-address
pub fn assignable_ip_range(cidr: &str) -> Option<(u32, u32)> {
    let (ip_str, prefix_str) = cidr.split_once('/')?;
    let ip = u32::from(ip_str.parse::<Ipv4Addr>().ok()?);
    let prefix_len = prefix_str.parse::<u32>().ok()?;
    if prefix_len > 30 {
        return None;
    }

    let mask = u32::MAX.checked_shl(32 - prefix_len).unwrap_or(0);
    let network_address = ip & mask;
    let broadcast_address = network_address | !mask;

    Some((network_address + 2, broadcast_address - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_unique_violation_of() {
        // sqlite lists the columns of the violated index
        let message = "UNIQUE constraint failed: addresses.mac_address";
        assert!(is_unique_violation_of(message, "addresses", "mac_address"));
        assert!(!is_unique_violation_of(message, "addresses", "tap_name"));
        let message = "UNIQUE constraint failed: addresses.network_uuid, addresses.internal_ip";
        assert!(is_unique_violation_of(message, "addresses", "internal_ip"));
        assert!(!is_unique_violation_of(message, "addresses", "mac_address"));

        // mysql names the violated index, and the violating value must not be mistaken for it
        let message = "Duplicate entry 'tap_name' for key 'addresses.addresses_active_mac_address'";
        assert!(is_unique_violation_of(message, "addresses", "mac_address"));
        assert!(!is_unique_violation_of(message, "addresses", "tap_name"));
        let message = "Duplicate entry 'abc-10.0.0.2' for key \
                       'addresses.addresses_active_network_internal_ip'";
        assert!(is_unique_violation_of(message, "addresses", "internal_ip"));
        let message = "Duplicate entry 'abc-10.0.0.2' for key \
                       'floating_ips.floating_ips_active_network_internal_ip_addr'";
        assert!(!is_unique_violation_of(
            message,
            "floating_ips",
            "floating_ip_addr"
        ));
        assert!(is_unique_violation_of(
            message,
            "floating_ips",
            "internal_ip_addr"
        ));
        let message = "Duplicate entry 'abc' for key 'addresses.PRIMARY'";
        assert!(!is_unique_violation_of(message, "addresses", "mac_address"));
    }

    #[test]
    fn test_assignable_ip_range() {
        let range = |first: Ipv4Addr, last: Ipv4Addr| Some((u32::from(first), u32::from(last)));

        assert_eq!(
            assignable_ip_range("192.168.100.0/24"),
            range(
                Ipv4Addr::new(192, 168, 100, 2),
                Ipv4Addr::new(192, 168, 100, 254)
            )
        );
        // host-bits of the address are ignored
        assert_eq!(
            assignable_ip_range("10.1.2.3/16"),
            range(Ipv4Addr::new(10, 1, 0, 2), Ipv4Addr::new(10, 1, 255, 254))
        );
        // smallest possible network with exactly one assignable address
        assert_eq!(
            assignable_ip_range("10.0.0.0/30"),
            range(Ipv4Addr::new(10, 0, 0, 2), Ipv4Addr::new(10, 0, 0, 2))
        );
        assert_eq!(
            assignable_ip_range("0.0.0.0/0"),
            range(Ipv4Addr::new(0, 0, 0, 2), Ipv4Addr::new(255, 255, 255, 254))
        );

        assert_eq!(assignable_ip_range("10.0.0.0/31"), None);
        assert_eq!(assignable_ip_range("10.0.0.0/33"), None);
        assert_eq!(assignable_ip_range("10.0.0.0"), None);
        assert_eq!(assignable_ip_range("10.0.0/24"), None);
        assert_eq!(assignable_ip_range("10.0.0.0/abc"), None);
    }
}
