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

#[derive(Debug, Clone, Serialize)]
pub struct CpuThread {
    pub id: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuCore {
    pub threads: Vec<CpuThread>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CpuPackage {
    pub cores: Vec<CpuCore>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Host {
    pub packages: Vec<CpuPackage>,
    pub hyperthreading_enabled: bool,
}

impl Default for CpuThread {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuThread {
    pub fn new() -> Self {
        CpuThread { id: 0 }
    }
}

impl Default for CpuCore {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuCore {
    pub fn new() -> Self {
        CpuCore {
            threads: Vec::new(),
        }
    }
}

impl Default for CpuPackage {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuPackage {
    pub fn new() -> Self {
        CpuPackage { cores: Vec::new() }
    }
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    pub fn new() -> Self {
        Host {
            hyperthreading_enabled: false,
            packages: Vec::new(),
        }
    }
}

pub fn init_host() -> io::Result<Host> {
    let mut host = Host::new();

    match is_hyperthreading_enabled() {
        Ok(enabled) => host.hyperthreading_enabled = enabled,
        Err(_) => host.hyperthreading_enabled = false,
    }

    let number_of_packages = get_number_of_cpu_packages()?;
    let number_of_threads = get_number_of_cpu_threads()?;

    let mut number_of_cores = number_of_threads;
    if host.hyperthreading_enabled {
        number_of_cores /= 2;
    }

    // TODO: also handle the case of unidentical big cpus
    let cores_per_package = number_of_cores / number_of_packages;

    // initalize vectors
    host.packages.resize(number_of_packages, CpuPackage::new());
    for package in host.packages.iter_mut() {
        package.cores.resize(cores_per_package, CpuCore::new());
    }

    // fill thread-ids into the structure
    for thread_id in 0..number_of_threads {
        let core_id = get_core_id(thread_id)?;
        let patckage_id = get_package_id(thread_id)?;

        host.packages[patckage_id].cores[core_id]
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
