// Copyright 2022 Tobias Anker <tobias.anker@kitsunemimi.moe>

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use sysinfo::{System, RefreshKind, MemoryRefreshKind};

pub fn get_total_memory_amount() -> u64 {
    let mut sys = System::new_all();
    sys.refresh_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
    sys.total_memory()
}

pub fn get_free_memory_amount() -> u64 {
    let mut sys = System::new_all();
    sys.refresh_specifics(
        RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()),
    );
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
