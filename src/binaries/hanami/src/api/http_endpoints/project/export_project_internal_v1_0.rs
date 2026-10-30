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

use actix_web::web::Json;
use actix_web::web::Path;
use apistos::api_operation;

use crate::database::address_table;
use crate::database::floating_ip_table;
use crate::database::meta_virtual_machine_table;
use crate::database::network_filter_table::{self, NetworkFilterEntry};
use crate::database::network_table;

use ainari_api::common_functions::*;
use ainari_api::error_mapping::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::exports::{
    FloatingIpExport, HanamiProjectExport, HanamiVirtualMachineExport, NetworkExport,
    NetworkFilterExport,
};
use ainari_api_structs::user_context::UserContext;

#[api_operation(
    tag = "project",
    summary = "Export project",
    description = r###"Export the networks, virtual_machines, packet filters and floating ip-addresses
of a project for a backup.

The virtual_machines are exported with their resources and their address within the network.
Their disk and their config are exported by sakura. The floating ip-addresses are exported with
the virtual_machine, which they are attached to. The vm-types are not exported, because they are
global and not bound to a project. Users, which are not admin, can only export their own project.

This is an internal call, which is protected by the internal api-key."###,
    error_code = 400,
    error_code = 401,
    error_code = 403,
    error_code = 500
)]
pub async fn export_project_internal(
    project_id: Path<String>,
    context: UserContext,
) -> Result<Json<HanamiProjectExport>, ErrorResponse> {
    check_project_access(&project_id, &context)?;

    let networks = network_table::list_networks_of_project(&project_id)
        .map_err(|e| map_db_list_error("networks", e, Some(&context)))?
        .into_iter()
        .map(|network| NetworkExport {
            uuid: network.uuid,
            name: network.name,
            subnet: network.subnet,
            disable_encryption: network.disable_encryption,
        })
        .collect();

    let meta_virtual_machines =
        meta_virtual_machine_table::list_meta_virtual_machines_of_project(&project_id)
            .map_err(|e| map_db_list_error("virtual_machines", e, Some(&context)))?;
    let mut virtual_machines = Vec::new();
    for virtual_machine in meta_virtual_machines {
        let address = address_table::get_address_of_virtual_machine(&virtual_machine.uuid)
            .map_err(|e| {
                map_db_uuid_get_delete_error(
                    "address of virtual_machine",
                    &virtual_machine.uuid,
                    e,
                    Some(&context),
                )
            })?;
        virtual_machines.push(HanamiVirtualMachineExport {
            uuid: virtual_machine.uuid,
            name: virtual_machine.name,
            number_of_cores: virtual_machine.number_of_cores,
            memory_size: virtual_machine.memory_size,
            disk_size: virtual_machine.disk_size,
            network_uuid: address.network_uuid,
            internal_ip: address.internal_ip,
            mac_address: address.mac_address,
        });
    }

    let network_filters = network_filter_table::list_network_filters_of_project(&project_id)
        .map_err(|e| map_db_list_error("network_filters", e, Some(&context)))?
        .into_iter()
        .map(|entry| to_filter_export(entry, &context))
        .collect::<Result<Vec<_>, _>>()?;

    let floating_ip_entries = floating_ip_table::list_floating_ips_of_project(&project_id)
        .map_err(|e| map_db_list_error("floating_ips", e, Some(&context)))?;
    let mut floating_ips = Vec::new();
    for floating_ip in floating_ip_entries {
        // the floating ip-address is attached to the address of a virtual_machine
        let virtual_machine_uuid = match (floating_ip.network_uuid, floating_ip.internal_ip_addr) {
            (Some(network_uuid), Some(internal_ip)) => {
                address_table::get_address_by_internal_ip(&network_uuid, &internal_ip)
                    .ok()
                    .and_then(|address| address.virtual_machine_uuid)
            }
            _ => None,
        };
        floating_ips.push(FloatingIpExport {
            uuid: floating_ip.uuid,
            name: floating_ip.name,
            floating_ip: floating_ip.floating_ip_addr,
            virtual_machine_uuid,
        });
    }

    Ok(Json(HanamiProjectExport {
        networks,
        virtual_machines,
        network_filters,
        floating_ips,
    }))
}

/// Converts a packet filter of the database into its export.
///
/// # Arguments
///
/// * `entry` - The packet filter from the database
/// * `context` - User-context of the request
///
/// # Returns
///
/// * `Ok(NetworkFilterExport)` - The exported packet filter
/// * `Err(ErrorResponse::InternalError)` - The database contains an invalid filter
fn to_filter_export(
    entry: NetworkFilterEntry,
    context: &UserContext,
) -> Result<NetworkFilterExport, ErrorResponse> {
    let invalid = |e: String| {
        ainari_api::report_internal_error!(
            Some(context),
            "Invalid packet-filter '{}' in database: {e}",
            entry.uuid
        );
        ErrorResponse::InternalError("Internal Error".to_string())
    };
    Ok(NetworkFilterExport {
        virtual_machine_uuid: entry.virtual_machine_uuid,
        direction: entry.direction.parse().map_err(invalid)?,
        ip_ranges: entry
            .ip_range_list()
            .iter()
            .map(|rule| rule.parse())
            .collect::<Result<_, _>>()
            .map_err(invalid)?,
        ports: entry
            .port_list()
            .iter()
            .map(|rule| rule.parse())
            .collect::<Result<_, _>>()
            .map_err(invalid)?,
    })
}
