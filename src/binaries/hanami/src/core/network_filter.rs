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

use std::net::Ipv4Addr;

use futures::lock::Mutex;
use uuid::Uuid;

use crate::config;
use crate::core::routing::torii_of_host;
use crate::database::address_table;
use crate::database::meta_virtual_machine_table;
use crate::database::network_filter_table;
use crate::database::network_filter_table::NetworkFilterEntry;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::network_filter_structs::*;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_common::config::Endpoint;

/// Serializes the changes of the packet filters.
///
/// Each change stores the include-lists, which the torii reports back, in the database. Without
/// the lock the answer of an older change could overwrite the one of a newer change in the
/// database, while the torii already applies the newer one. It has to be held from the request
/// to the torii until the answer is stored.
pub static NETWORK_FILTER_LOCK: Mutex<()> = Mutex::new(());

/// Address of a virtual_machine on the torii of its host.
pub struct FilterTarget {
    /// The torii of the host, which runs the virtual_machine
    pub torii: Endpoint,
    /// Tenant of the network of the virtual_machine
    pub vni: u32,
    /// Internal address of the virtual_machine
    pub ip: Ipv4Addr,
}

/// Resolves a virtual_machine to its address on the torii of its host, before its packet filter
/// is changed.
///
/// The virtual_machine is resolved to its internal address, the tenant of its network and its
/// host by the address-table. The torii of that host owns the route and the TAP-device of the
/// virtual_machine, so it applies the filter.
///
/// # Arguments
/// * `virtual_machine_uuid` - The UUID of the virtual_machine
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(FilterTarget)` with the torii, the tenant and the internal address of the
///   virtual_machine
/// * `Err(ErrorResponse)` if the user is only allowed to read, the virtual_machine doesn't exist,
///   the user is not allowed to access it or its host is unknown
pub async fn resolve_filter_target(
    virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<FilterTarget, ErrorResponse> {
    // observers without admin-privileges are only allowed to read, which is checked before the
    // torii is changed
    if context.is_read_only() {
        return Err(permission_denied_response());
    }

    // check, that the virtual_machine exists and the user is allowed to access it
    meta_virtual_machine_table::get_meta_virtual_machine(virtual_machine_uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("virtual_machine", virtual_machine_uuid, e))?;

    let address =
        address_table::get_address_of_virtual_machine(virtual_machine_uuid).map_err(|e| {
            map_db_uuid_get_delete_error("address of virtual_machine", virtual_machine_uuid, e)
        })?;

    // entries of a database, which was created before the host-address was stored with the
    // addresses, have no torii, which could filter their traffic
    if address.host_address.is_empty() {
        log::error!(
            "Address '{}' of virtual_machine '{virtual_machine_uuid}' has no host-address.",
            address.uuid
        );
        return Err(ErrorResponse::InternalError("Internal Error".to_string()));
    }

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    Ok(FilterTarget {
        torii: torii_of_host(&endpoints.torii, &address.host_address)?,
        vni: address.vni,
        ip: address.internal_ip,
    })
}

/// Stores the packet filter, which the torii reported back after a change, in the database.
///
/// The torii already applies the filter at this point. If it can not be stored, the view of
/// hanami is outdated until the next change of the filter, which stores the complete
/// include-lists of the torii again.
///
/// # Arguments
/// * `virtual_machine_uuid` - The UUID of the virtual_machine
/// * `direction` - The direction of the filter
/// * `filter_resp` - The answer of the torii with the complete include-lists
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(NetworkFilterEntry)` with the stored filter, which is marked as deleted, if both
///   include-lists are empty now
/// * `Err(ErrorResponse)` if the filter could not be stored
pub fn store_network_filter(
    virtual_machine_uuid: &Uuid,
    direction: FilterDirection,
    filter_resp: &FilterResp,
    context: &UserContext,
) -> Result<NetworkFilterEntry, ErrorResponse> {
    network_filter_table::set_network_filter(
        virtual_machine_uuid,
        direction,
        &filter_resp.filter,
        context,
    )
    .map_err(|e| {
        log::error!(
            "The {direction} packet-filter of virtual_machine '{virtual_machine_uuid}' was \
             changed in the torii, but could not be stored in the database."
        );
        map_db_uuid_get_delete_error("network-filter", virtual_machine_uuid, e)
    })
}

/// Converts a packet filter of the database into the response of the api.
///
/// # Arguments
/// * `entry` - The packet filter from the database
///
/// # Returns
/// The response, which is sent back to the user, or an internal error, if the stored direction
/// is invalid
pub fn to_network_filter_resp(
    entry: NetworkFilterEntry,
) -> Result<NetworkFilterResp, ErrorResponse> {
    let direction = entry.direction.parse().map_err(|e| {
        log::error!("Invalid packet-filter '{}' in database: {e}", entry.uuid);
        ErrorResponse::InternalError("Internal Error".to_string())
    })?;

    Ok(NetworkFilterResp {
        uuid: entry.uuid,
        virtual_machine_uuid: entry.virtual_machine_uuid,
        direction,
        ip_ranges: entry.ip_range_list(),
        ports: entry.port_list(),
        created_at: entry.created_at,
        created_by: entry.created_by,
        updated_at: entry.updated_at,
        updated_by: entry.updated_by,
    })
}
