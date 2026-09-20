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

use rand::prelude::IndexedRandom;

use crate::database::host_table;
use crate::database::host_table::HostEntry;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

/// Selects the onsen-host, on which new files are stored.
///
/// The host is picked randomly out of all registered hosts. This is not a real scheduling yet, so
/// neither the free space nor the load of the hosts is taken into account.
///
/// # Arguments
///
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(HostEntry)` - The selected onsen-host.
/// * `Err(ErrorResponse::InternalError)` - The hosts could not be read or no host is registered.
pub fn select_onsen(context: &UserContext) -> Result<HostEntry, ErrorResponse> {
    // list all available hosts
    let hosts = match host_table::list_hosts(context) {
        Ok(hosts) => hosts,
        Err(e) => {
            log::error!("Failed to get list of hosts form database: '{e}'");
            return Err(ErrorResponse::InternalError("Internal Error".to_string()));
        }
    };

    // check that there is at least one host
    if hosts.is_empty() {
        log::error!("No hosts to schedule new instance.");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    // pick one of the hosts at random
    let mut rng = rand::rng();
    let selected_host = if let Some(host) = hosts.choose(&mut rng) {
        host.clone()
    } else {
        log::error!("Failed to select a host out of the list of available hosts.");
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    };

    Ok(selected_host)
}
