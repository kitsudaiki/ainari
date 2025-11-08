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

use crate::database::host_table;
use crate::database::host_table::HostEntry;

use ainari_api::errors::ErrorResponse;
use ainari_api_structs::user_context::UserContext;

pub fn select_onsen(context: &UserContext) -> Result<HostEntry, ErrorResponse> {
    // list all avaialble hosts
    let hosts = match host_table::list_hosts(context) {
        Ok(hosts) => hosts,
        Err(e) => {
            log::error!("Failed to get list of hosts form database: '{e}'");
            return Err(ErrorResponse::InternalError("".to_string()));
        }
    };

    // check that there is at least one host
    if hosts.is_empty() {
        log::error!("No hosts to schedule new cluster.");
        return Err(ErrorResponse::InternalError("".to_string()));
    }

    // select first host
    // TODO: also be able to select one of many hosts
    let selected_host = if let Some(host) = hosts.first() {
        host.clone()
    } else {
        log::error!("No hosts with list-position 0 doesn't exist.");
        return Err(ErrorResponse::InternalError("".to_string()));
    };

    Ok(selected_host)
}
