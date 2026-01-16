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

use crate::cpu::*;
use serde::Serialize;
use std::io::{self};

/// Represents a single CPU thread.
#[derive(Debug, Clone, Serialize)]
pub struct CpuThread {
    /// Unique identifier for the thread
    pub id: usize,
}

/// Represents a single CPU core containing multiple threads.
#[derive(Debug, Clone, Serialize)]
pub struct CpuCore {
    /// Collection of threads belonging to this core
    pub threads: Vec<CpuThread>,
}

/// Represents a CPU package containing multiple cores.
#[derive(Debug, Clone, Serialize)]
pub struct CpuPackage {
    /// Collection of cores belonging to this package
    pub cores: Vec<CpuCore>,
}

/// Represents the entire host CPU system with multiple packages.
#[derive(Debug, Clone, Serialize)]
pub struct Host {
    /// Collection of CPU packages in the system
    pub packages: Vec<CpuPackage>,
    /// Indicates whether hyperthreading is enabled in the system
    pub hyperthreading_enabled: bool,
}

impl Default for CpuThread {
    /// Creates a new CpuThread with default values.
    fn default() -> Self {
        Self::new()
    }
}

impl CpuThread {
    /// Creates a new CpuThread with thread ID set to 0.
    ///
    /// # Returns
    /// A new CpuThread instance with default values.
    pub fn new() -> Self {
        CpuThread { id: 0 }
    }
}

impl Default for CpuCore {
    /// Creates a new CpuCore with default values.
    fn default() -> Self {
        Self::new()
    }
}

impl CpuCore {
    /// Creates a new CpuCore with an empty threads collection.
    ///
    /// # Returns
    /// A new CpuCore instance with an empty threads vector.
    pub fn new() -> Self {
        CpuCore {
            threads: Vec::new(),
        }
    }
}

impl Default for CpuPackage {
    /// Creates a new CpuPackage with default values.
    fn default() -> Self {
        Self::new()
    }
}

impl CpuPackage {
    /// Creates a new CpuPackage with an empty cores collection.
    ///
    /// # Returns
    /// A new CpuPackage instance with an empty cores vector.
    pub fn new() -> Self {
        CpuPackage { cores: Vec::new() }
    }
}

impl Default for Host {
    /// Creates a new Host with default values.
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    /// Creates a new Host with hyperthreading disabled and empty packages collection.
    ///
    /// # Returns
    /// A new Host instance with default values.
    pub fn new() -> Self {
        Host {
            hyperthreading_enabled: false,
            packages: Vec::new(),
        }
    }
}

/// Initializes and configures the host CPU structure.
///
/// This function queries system information to determine the CPU configuration,
/// including the number of packages, cores, and threads, and organizes them
/// into the appropriate data structure.
///
/// # Returns
/// - `Ok(Host)`: A configured Host structure representing the system's CPU
/// - `Err(io::Error)`: If there was an error reading CPU information
pub fn init_host() -> io::Result<Host> {
    let mut host = Host::new();

    // Set hyperthreading status, defaulting to false if detection fails
    match is_hyperthreading_enabled() {
        Ok(enabled) => host.hyperthreading_enabled = enabled,
        Err(_) => host.hyperthreading_enabled = false,
    }

    // Get basic CPU topology information
    let number_of_packages = get_number_of_cpu_packages()?;
    let number_of_threads = get_number_of_cpu_threads()?;

    // Calculate number of physical cores, accounting for hyperthreading
    let mut number_of_cores = number_of_threads;
    if host.hyperthreading_enabled {
        number_of_cores /= 2;
    }

    // TODO: also handle the case of unidentical big cpus
    // Calculate average cores per package (this assumes identical packages)
    let cores_per_package = number_of_cores / number_of_packages;

    // Initialize the package and core structures
    host.packages.resize(number_of_packages, CpuPackage::new());
    for package in host.packages.iter_mut() {
        package.cores.resize(cores_per_package, CpuCore::new());
    }

    // Populate the thread IDs into the structure based on their core and package affiliation
    for thread_id in 0..number_of_threads {
        let core_id = get_core_id(thread_id)?;
        let package_id = get_package_id(thread_id)?; // Fixed typo in variable name

        // Add the thread to its corresponding core and package
        host.packages[package_id].cores[core_id]
            .threads
            .push(CpuThread { id: thread_id });
    }

    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_host() {
        let host = init_host().unwrap();

        let j = serde_json::to_string_pretty(&host).unwrap();
        println!("{j}");
    }
}
