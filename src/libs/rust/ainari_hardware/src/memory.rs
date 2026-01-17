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

use sysinfo::{MemoryRefreshKind, RefreshKind, System};

/// Retrieves the total amount of physical memory installed in the system.
///
/// This function initializes a system information handler and refreshes memory-specific
/// information to get the most up-to-date value. It returns the total memory in bytes.
///
/// # Returns
///
/// * `u64` - Total amount of physical memory in bytes
pub fn get_total_memory_amount() -> u64 {
    // Create a new system information handler with all capabilities
    let mut sys = System::new_all();

    // Refresh only memory-related information for accurate readings
    sys.refresh_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()));

    // Return the total memory amount in bytes
    sys.total_memory()
}

/// Retrieves the amount of free physical memory available in the system.
///
/// This function initializes a system information handler and refreshes memory-specific
/// information to get the most current value of available memory. It returns the free
/// memory in bytes.
///
/// # Returns
///
/// * `u64` - Amount of available memory in bytes
pub fn get_free_memory_amount() -> u64 {
    // Create a new system information handler with all capabilities
    let mut sys = System::new_all();

    // Refresh only memory-related information for accurate readings
    sys.refresh_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()));

    // Return the available memory amount in bytes
    sys.available_memory()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu() {
        println!("Total amount of memory: {}", get_total_memory_amount());
        println!("Free amount of memory: {}", get_free_memory_amount());
    }
}
