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

use uuid::Uuid;

use crate::config;
use crate::database::address_table;
use crate::database::floating_ip_table;
use crate::database::floating_ip_table::{FloatingIpAttachError, FloatingIpEntry};
use crate::database::meta_virtual_machine_table;

use ainari_api::common_functions::*;
use ainari_api::errors::ErrorResponse;
use ainari_api_structs::floating_ip_structs::FloatingIpResp;
use ainari_api_structs::user_context::UserContext;
use ainari_clients::endpoints::get_endpoints;
use ainari_clients::floating_ip as floating_ip_clients;

/// Attaches a floating ip-address to a virtual_machine.
///
/// The network, the internal ip-address and the tenant of the virtual_machine are read from its
/// address in the database. The floating ip-address is claimed in the database first, so two
/// requests can not attach it at the same time, and registered in the torii afterwards. If the
/// registration fails, the claim is released again.
///
/// The tenant of the internal address travels with the registration. A floating ip-address is
/// unique across all networks, so it is what tells the torii, which tenant an arriving packet
/// belongs to - and the internal address behind it is only meaningful within that tenant.
///
/// # Arguments
/// * `floating_ip_uuid` - UUID of the floating ip-address to attach
/// * `virtual_machine_uuid` - UUID of the virtual_machine, which the floating ip-address is attached to
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(FloatingIpEntry)` with the attached floating ip-address
/// * `Err(ErrorResponse)` with an appropriate error on failure
pub async fn attach_floating_ip(
    floating_ip_uuid: &Uuid,
    virtual_machine_uuid: &Uuid,
    context: &UserContext,
) -> Result<FloatingIpEntry, ErrorResponse> {
    // check, that the virtual_machine exists and the user is allowed to access it
    meta_virtual_machine_table::get_meta_virtual_machine(virtual_machine_uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("virtual_machine", virtual_machine_uuid, e))?;

    let address =
        address_table::get_address_of_virtual_machine(virtual_machine_uuid).map_err(|e| {
            map_db_uuid_get_delete_error("address of virtual_machine", virtual_machine_uuid, e)
        })?;

    let floating_ip_entry = floating_ip_table::attach_floating_ip(
        floating_ip_uuid,
        &address.network_uuid,
        &address.internal_ip,
        context,
    )
    .map_err(|e| match e {
        FloatingIpAttachError::NotFound => {
            ErrorResponse::NotFound(format!("Floating ip '{floating_ip_uuid}' not found."))
        }
        FloatingIpAttachError::AlreadyAttached => ErrorResponse::Conflict(format!(
            "Floating ip '{floating_ip_uuid}' is already attached to a virtual_machine."
        )),
        FloatingIpAttachError::VirtualMachineHasFloatingIp => ErrorResponse::Conflict(format!(
            "Virtual_machine '{virtual_machine_uuid}' already has a floating ip."
        )),
        FloatingIpAttachError::InternalError => {
            log::error!("Failed to attach floating ip '{floating_ip_uuid}' in database.");
            ErrorResponse::InternalError("Internal Error".to_string())
        }
    })?;

    let result = async {
        let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
            .await
            .map_err(map_ainari_error_to_api_response)?;

        floating_ip_clients::create_floating_ip(
            &endpoints.torii,
            &context.token,
            &config::INTERNAL_API_KEY,
            &floating_ip_entry.name,
            &address.network_uuid,
            &floating_ip_entry.floating_ip_addr,
            &address.internal_ip,
            address.vni,
            config::CONFIG.skip_tls_verification,
        )
        .await
        .map_err(map_ainari_error_to_api_response)
    }
    .await;

    if let Err(e) = result {
        log::error!(
            "Failed to register floating ip '{}' in torii. Detaching it again.",
            floating_ip_entry.floating_ip_addr
        );
        let _ = floating_ip_table::detach_floating_ip(floating_ip_uuid, context);
        return Err(e);
    }

    Ok(floating_ip_entry)
}

/// Detaches a floating ip-address from its virtual_machine.
///
/// The NAT of the floating ip-address is removed from the torii first and the database-entry is
/// updated afterwards, so the floating ip-address is never attachable again, while the torii
/// still translates it. A floating ip-address, which is not attached, is left untouched.
///
/// There is no permission-check, so the caller has to verify, that the user is allowed to
/// detach the floating ip-address.
///
/// # Arguments
/// * `floating_ip_entry` - The floating ip-address to detach
/// * `context` - User context containing authentication information
///
/// # Returns
/// * `Ok(())` if the floating ip-address is detached
/// * `Err(ErrorResponse)` with an appropriate error on failure
pub async fn detach_floating_ip(
    floating_ip_entry: &FloatingIpEntry,
    context: &UserContext,
) -> Result<(), ErrorResponse> {
    if floating_ip_entry.internal_ip_addr.is_none() {
        return Ok(());
    }

    let endpoints = get_endpoints(&config::CONFIG.miko, config::CONFIG.skip_tls_verification)
        .await
        .map_err(map_ainari_error_to_api_response)?;

    floating_ip_clients::delete_floating_ip(
        &endpoints.torii,
        &context.token,
        &config::INTERNAL_API_KEY,
        &floating_ip_entry.floating_ip_addr,
        config::CONFIG.skip_tls_verification,
    )
    .await
    .map_err(map_ainari_error_to_api_response)?;

    floating_ip_table::detach_floating_ip(&floating_ip_entry.uuid, context)
        .map_err(|e| map_db_uuid_get_delete_error("floating_ip", &floating_ip_entry.uuid, e))
}

/// Converts a floating ip-address of the database into the response of the api.
///
/// # Arguments
/// * `floating_ip_entry` - The floating ip-address from the database
///
/// # Returns
/// The response, which is sent back to the user
pub fn to_floating_ip_resp(floating_ip_entry: FloatingIpEntry) -> FloatingIpResp {
    FloatingIpResp {
        uuid: floating_ip_entry.uuid,
        network_uuid: floating_ip_entry.network_uuid,
        floating_ip: floating_ip_entry.floating_ip_addr,
        internal_ip: floating_ip_entry.internal_ip_addr,
        created_by: floating_ip_entry.created_by,
        created_at: floating_ip_entry.created_at,
        updated_by: floating_ip_entry.updated_by,
        updated_at: floating_ip_entry.updated_at,
    }
}
