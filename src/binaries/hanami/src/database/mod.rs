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

use std::net::Ipv4Addr;

/// Initializes all database tables required for the application.
///
/// This function orchestrates the initialization of all database tables
/// in the correct order. If any table fails to initialize, the entire
/// operation fails and returns an error.
///
/// # Returns
/// * `Ok(())` - All tables initialized successfully
/// * `Err(Box<dyn std::error::Error>)` - One or more tables failed to initialize
pub fn init_database() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize host-table
    match host_table::init_host_table() {
        Ok(_) => log::info!("Initialized host-database-table"),
        Err(e) => {
            log::error!("Failed to initialize host-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize meta virtual_machine table
    match meta_virtual_machine_table::init_meta_virtual_machine_table() {
        Ok(_) => log::info!("Initialized virtual_machine-database-table"),
        Err(e) => {
            log::error!("Failed to initialize virtual_machine-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize network table
    match network_table::init_network_table() {
        Ok(_) => log::info!("Initialized network-database-table"),
        Err(e) => {
            log::error!("Failed to initialize network-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize floating-ip table
    match floating_ip_table::init_floating_ip_table() {
        Ok(_) => log::info!("Initialized floating-ip-database-table"),
        Err(e) => {
            log::error!("Failed to initialize floating-ip-database-table: {e}");
            return Err(e);
        }
    };

    // Initialize address table
    match address_table::init_address_table() {
        Ok(_) => log::info!("Initialized address-database-table"),
        Err(e) => {
            log::error!("Failed to initialize address-database-table: {e}");
            return Err(e);
        }
    };

    Ok(())
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
